//! Minimal polling RTL8139 Ethernet driver and IPv4/ARP/ICMP support.
//!
//! The default QEMU machine can use `-netdev user,id=n0 -device rtl8139,netdev=n0`.
//! This intentionally starts with static IPv4 configuration, DNS A lookups,
//! and ICMP echo; it provides a small, debuggable base for later socket support.

extern crate alloc;
use alloc::vec::Vec;
use crate::arch::io::{inb, inl, inw, outb, outl, outw};
use crate::drivers::timer::timer_get_ticks;

const PCI_ADDR: u16 = 0xCF8;
const PCI_DATA: u16 = 0xCFC;
const RX_RING_SIZE: usize = 8192 + 16 + 1500;
const RX_CAPACITY: usize = 2048;
const TX_CAPACITY: usize = 1536;
const IP: [u8; 4] = [10, 0, 2, 15];
const GATEWAY: [u8; 4] = [10, 0, 2, 2];
const DNS_SERVER: [u8; 4] = [10, 0, 2, 3];
const MASK: [u8; 4] = [255, 255, 255, 0];

static mut IO_BASE: u16 = 0;
static mut MAC: [u8; 6] = [0; 6];
static mut RX_OFFSET: usize = 0;
static mut TX_INDEX: usize = 0;
static mut RX_READY: bool = false;
#[repr(align(256))]
struct RxBuffer([u8; RX_RING_SIZE]);
#[repr(align(256))]
struct TxBuffers([[u8; TX_CAPACITY]; 4]);
static mut RX_BUFFER: RxBuffer = RxBuffer([0; RX_RING_SIZE]);
static mut TX_BUFFERS: TxBuffers = TxBuffers([[0; TX_CAPACITY]; 4]);

fn pci_read(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    let address = 0x8000_0000u32 | ((bus as u32) << 16) | ((slot as u32) << 11)
        | ((function as u32) << 8) | ((offset as u32) & 0xFC);
    unsafe { outl(PCI_ADDR, address); inl(PCI_DATA) }
}

pub fn network_init() -> bool {
    let mut found = None;
    for bus in 0..=255u16 {
        for slot in 0..32u8 {
            let id = pci_read(bus as u8, slot, 0, 0);
            if id as u16 == 0x10EC && (id >> 16) as u16 == 0x8139 {
                found = Some((bus as u8, slot));
                break;
            }
        }
        if found.is_some() { break; }
    }
    let Some((bus, slot)) = found else { return false; };
    let bar = pci_read(bus, slot, 0, 0x10);
    if bar & 1 == 0 { return false; }
    let base = (bar & 0xFFFC) as u16;
    unsafe {
        IO_BASE = base;
        // Enable I/O space and bus mastering.
        outl(PCI_ADDR, 0x8000_0000 | ((bus as u32) << 16) | ((slot as u32) << 11) | 4);
        let command = inw(PCI_DATA as u16);
        outw(PCI_DATA, command | 0x0005);
        outb(base + 0x52, 0x00); // power on
        outb(base + 0x37, 0x10); // reset
        let mut spins = 1_000_000;
        while inb(base + 0x37) & 0x10 != 0 && spins != 0 { spins -= 1; }
        if spins == 0 { return false; }
        for i in 0..6 { MAC[i] = inb(base + i as u16); }
        let rx_buffer = core::ptr::addr_of_mut!(RX_BUFFER.0) as *mut u8;
        core::ptr::write_bytes(rx_buffer, 0, RX_RING_SIZE);
        outl(base + 0x30, rx_buffer as u32);
        outw(base + 0x3C, 0x0000); // disable device interrupts; packets are polled
        outl(base + 0x44, 0x0000_008F); // accept broadcast, multicast, physical, all runt/long
        outb(base + 0x37, 0x0C); // enable receiver and transmitter
        RX_OFFSET = 0;
        TX_INDEX = 0;
        RX_READY = true;
    }
    true
}

pub fn is_available() -> bool { unsafe { RX_READY } }
pub fn mac_address() -> [u8; 6] { unsafe { MAC } }
pub fn ip_address() -> [u8; 4] { IP }
pub fn netmask() -> [u8; 4] { MASK }

fn transmit(frame: &[u8]) -> bool {
    if frame.len() > TX_CAPACITY || !is_available() { return false; }
    unsafe {
        let index = TX_INDEX & 3;
        let buffer = (core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut u8).add(index * TX_CAPACITY);
        core::ptr::copy_nonoverlapping(frame.as_ptr(), buffer, frame.len());
        let base = IO_BASE;
        outl(base + 0x20 + (index as u16) * 4, buffer as u32);
        outl(base + 0x10 + (index as u16) * 4, frame.len() as u32);
        TX_INDEX = (index + 1) & 3;
    }
    true
}

fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() { sum += (data[i] as u32) << 8; }
    while sum >> 16 != 0 { sum = (sum & 0xFFFF) + (sum >> 16); }
    !(sum as u16)
}

fn put16(out: &mut [u8], offset: usize, value: u16) { out[offset..offset + 2].copy_from_slice(&value.to_be_bytes()); }

fn consume_rx(length: usize) {
    unsafe {
        let base = IO_BASE;
        RX_OFFSET = (RX_OFFSET + length + 4 + 3) & !3;
        if RX_OFFSET >= 8192 { RX_OFFSET %= 8192; }
        outw(base + 0x38, (RX_OFFSET - 16) as u16);
    }
}

fn poll_arp_reply(target: [u8; 4]) -> Option<[u8; 6]> {
    unsafe {
        let base = IO_BASE;
        if inb(base + 0x37) & 1 != 0 { return None; }
        let ring = core::ptr::addr_of!(RX_BUFFER.0) as *const u8;
        let offset = RX_OFFSET;
        let status = core::ptr::read_unaligned(ring.add(offset) as *const u16);
        let length = core::ptr::read_unaligned(ring.add(offset + 2) as *const u16) as usize;
        let mut found = None;
        if status & 1 != 0 && (42..=RX_CAPACITY).contains(&length) {
            let packet = core::slice::from_raw_parts(ring.add(offset + 4), length);
            if packet[12..14] == 0x0806u16.to_be_bytes()
                && packet[20..22] == 2u16.to_be_bytes()
                && packet[28..32] == target
            {
                let mut mac = [0u8; 6];
                mac.copy_from_slice(&packet[22..28]);
                found = Some(mac);
            }
        }
        consume_rx(length);
        found
    }
}

fn resolve_mac(address: [u8; 4], timeout_ticks: u32) -> Result<[u8; 6], &'static str> {
    if !send_arp_request(address) { return Err("could not send ARP request"); }
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks {
        if let Some(mac) = poll_arp_reply(address) { return Ok(mac); }
    }
    Err("ARP timed out; check QEMU networking")
}

fn encode_dns_name(name: &str, output: &mut [u8]) -> Option<usize> {
    let name = name.strip_suffix('.').unwrap_or(name);
    if name.is_empty() || name.len() > 253 { return None; }
    let mut cursor = 0;
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 || cursor + label.len() + 1 >= output.len() { return None; }
        output[cursor] = label.len() as u8;
        cursor += 1;
        for byte in label.bytes() {
            if !byte.is_ascii_alphanumeric() && byte != b'-' { return None; }
            output[cursor] = byte.to_ascii_lowercase();
            cursor += 1;
        }
    }
    output[cursor] = 0;
    Some(cursor + 1)
}

fn skip_dns_name(packet: &[u8], cursor: &mut usize) -> Option<()> {
    loop {
        let length = *packet.get(*cursor)?;
        if length & 0xC0 == 0xC0 {
            packet.get(*cursor + 1)?;
            *cursor += 2;
            return Some(());
        }
        *cursor += 1;
        if length == 0 { return Some(()); }
        if length & 0xC0 != 0 || length > 63 { return None; }
        *cursor = (*cursor).checked_add(length as usize)?;
        packet.get(*cursor - 1)?;
    }
}

fn parse_dns_a(packet: &[u8], id: u16) -> Option<[u8; 4]> {
    if packet.len() < 12 || u16::from_be_bytes([packet[0], packet[1]]) != id { return None; }
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    if flags & 0x8000 == 0 || flags & 0x000F != 0 { return None; }
    let questions = u16::from_be_bytes([packet[4], packet[5]]) as usize;
    let answers = u16::from_be_bytes([packet[6], packet[7]]) as usize;
    let mut cursor = 12;
    for _ in 0..questions {
        skip_dns_name(packet, &mut cursor)?;
        cursor = cursor.checked_add(4)?;
        packet.get(cursor - 1)?;
    }
    for _ in 0..answers {
        skip_dns_name(packet, &mut cursor)?;
        let header = packet.get(cursor..cursor.checked_add(10)?)?;
        let kind = u16::from_be_bytes([header[0], header[1]]);
        let class = u16::from_be_bytes([header[2], header[3]]);
        let data_len = u16::from_be_bytes([header[8], header[9]]) as usize;
        cursor += 10;
        let data = packet.get(cursor..cursor.checked_add(data_len)?)?;
        if kind == 1 && class == 1 && data_len == 4 { return Some([data[0], data[1], data[2], data[3]]); }
        cursor += data_len;
    }
    None
}

fn send_dns_query(name: &str, id: u16, server_mac: [u8; 6]) -> Result<(), &'static str> {
    let mut dns = [0u8; 512];
    put16(&mut dns, 0, id);
    put16(&mut dns, 2, 0x0100); // recursion desired
    put16(&mut dns, 4, 1);
    let name_len = encode_dns_name(name, &mut dns[12..]).ok_or("invalid hostname")?;
    let question_end = 12 + name_len;
    put16(&mut dns, question_end, 1); // A
    put16(&mut dns, question_end + 2, 1); // IN
    let dns_len = question_end + 4;
    let udp_len = 8 + dns_len;
    let ip_len = 20 + udp_len;
    let frame_len = 14 + ip_len;
    let mut frame = [0u8; 600];
    frame[0..6].copy_from_slice(&server_mac);
    frame[6..12].copy_from_slice(&mac_address());
    put16(&mut frame, 12, 0x0800);
    frame[14] = 0x45;
    put16(&mut frame, 16, ip_len as u16);
    frame[22] = 64;
    frame[23] = 17;
    frame[26..30].copy_from_slice(&IP);
    frame[30..34].copy_from_slice(&DNS_SERVER);
    let ip_checksum = checksum(&frame[14..34]);
    put16(&mut frame, 24, ip_checksum);
    put16(&mut frame, 34, 53000);
    put16(&mut frame, 36, 53);
    put16(&mut frame, 38, udp_len as u16);
    // A zero UDP checksum is valid for IPv4.
    frame[42..42 + dns_len].copy_from_slice(&dns[..dns_len]);
    if transmit(&frame[..frame_len]) { Ok(()) } else { Err("could not send DNS query") }
}

fn poll_dns_reply(id: u16) -> Option<Vec<u8>> {
    unsafe {
        let base = IO_BASE;
        if inb(base + 0x37) & 1 != 0 { return None; }
        let ring = core::ptr::addr_of!(RX_BUFFER.0) as *const u8;
        let offset = RX_OFFSET;
        let status = core::ptr::read_unaligned(ring.add(offset) as *const u16);
        let length = core::ptr::read_unaligned(ring.add(offset + 2) as *const u16) as usize;
        let mut result = None;
        if status & 1 != 0 && (42..=RX_CAPACITY).contains(&length) {
            let packet = core::slice::from_raw_parts(ring.add(offset + 4), length);
            if packet[12..14] == 0x0800u16.to_be_bytes() {
                let ip_header_len = ((packet[14] & 0x0F) as usize) * 4;
                if packet[23] == 17 && ip_header_len >= 20 && length >= 14 + ip_header_len + 8 {
                    let udp = 14 + ip_header_len;
                    let udp_len = u16::from_be_bytes([packet[udp + 4], packet[udp + 5]]) as usize;
                    if u16::from_be_bytes([packet[udp + 2], packet[udp + 3]]) == 53000
                        && udp_len >= 20 && udp + udp_len <= length
                    {
                        let dns = &packet[udp + 8..udp + udp_len];
                        if dns.len() >= 2 && u16::from_be_bytes([dns[0], dns[1]]) == id {
                            result = Some(dns.to_vec());
                        }
                    }
                }
            }
        }
        consume_rx(length);
        result
    }
}

pub fn resolve_ipv4(name: &str, timeout_ticks: u32) -> Result<[u8; 4], &'static str> {
    if !is_available() { return Err("no RTL8139 adapter found"); }
    let server_mac = resolve_mac(DNS_SERVER, timeout_ticks)?;
    let id = (timer_get_ticks() as u16) ^ 0x5057;
    send_dns_query(name, id, server_mac)?;
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks {
        if let Some(reply) = poll_dns_reply(id) {
            return parse_dns_a(&reply, id).ok_or("DNS response has no IPv4 address");
        }
    }
    Err("DNS lookup timed out")
}

fn send_arp_request(target: [u8; 4]) -> bool {
    let mut f = [0u8; 42];
    f[0..6].fill(0xFF); f[6..12].copy_from_slice(&mac_address()); put16(&mut f, 12, 0x0806);
    put16(&mut f, 14, 1); put16(&mut f, 16, 0x0800); f[18] = 6; f[19] = 4; put16(&mut f, 20, 1);
    f[22..28].copy_from_slice(&mac_address()); f[28..32].copy_from_slice(&IP); f[38..42].copy_from_slice(&target);
    transmit(&f)
}

fn send_echo_request(destination: [u8; 4], destination_mac: [u8; 6], sequence: u16) -> bool {
    let mut f = [0u8; 42];
    f[0..6].copy_from_slice(&destination_mac); f[6..12].copy_from_slice(&mac_address()); put16(&mut f, 12, 0x0800);
    f[14] = 0x45; put16(&mut f, 16, 28); f[22] = 64; f[23] = 1;
    f[26..30].copy_from_slice(&IP); f[30..34].copy_from_slice(&destination);
    let ip_checksum = checksum(&f[14..34]);
    put16(&mut f, 24, ip_checksum);
    f[34] = 8; put16(&mut f, 38, 0x5057); put16(&mut f, 40, sequence);
    let icmp_checksum = checksum(&f[34..42]);
    put16(&mut f, 36, icmp_checksum);
    transmit(&f)
}

/// Process one available Ethernet packet and return true for an ICMP echo reply.
fn poll_packet(target: [u8; 4], sequence: u16) -> bool {
    unsafe {
        let base = IO_BASE;
        let status = inb(base + 0x37);
        // Command.BUFE is set while the receive ring is empty.
        if status & 1 != 0 { return false; }
        let ring = core::ptr::addr_of!(RX_BUFFER.0) as *const u8;
        let offset = RX_OFFSET;
        let rx_status = core::ptr::read_unaligned(ring.add(offset) as *const u16);
        let length = core::ptr::read_unaligned(ring.add(offset + 2) as *const u16) as usize;
        let packet = ring.add(offset + 4);
        if rx_status & 1 != 0 && (14..=RX_CAPACITY).contains(&length) {
            let ether_type = u16::from_be_bytes([*packet.add(12), *packet.add(13)]);
            if ether_type == 0x0806 && length >= 42 {
                let p = core::slice::from_raw_parts(packet, length);
                // Only answer ARP requests addressed to this host. Replies are
                // consumed by the caller waiting for neighbor resolution.
                if p[20] == 0 && p[21] == 1 && p[38..42] == IP {
                    let mac = MAC;
                    let mut reply = [0u8; 42]; reply[0..6].copy_from_slice(&p[22..28]);
                    reply[6..12].copy_from_slice(&mac); put16(&mut reply, 12, 0x0806);
                    put16(&mut reply, 14, 1); put16(&mut reply, 16, 0x0800); reply[18] = 6; reply[19] = 4; put16(&mut reply, 20, 2);
                    reply[22..28].copy_from_slice(&mac); reply[28..32].copy_from_slice(&IP);
                    reply[32..38].copy_from_slice(&p[22..28]); reply[38..42].copy_from_slice(&p[28..32]);
                    let _ = transmit(&reply);
                }
            } else if ether_type == 0x0800 && length >= 42 {
                let p = core::slice::from_raw_parts(packet, length);
                if p[23] == 1 && p[34] == 0 && p[35] == 0 && p[38..40] == 0x5057u16.to_be_bytes()
                    && p[40..42] == sequence.to_be_bytes() && p[30..34] == IP && p[26..30] == target { return true; }
            }
        }
        RX_OFFSET = (offset + length + 4 + 3) & !3;
        if RX_OFFSET >= 8192 { RX_OFFSET %= 8192; }
        outw(base + 0x38, (RX_OFFSET - 16) as u16);
    }
    false
}

pub fn ping(destination: [u8; 4], timeout_ticks: u32) -> Result<(), &'static str> {
    if !is_available() { return Err("no RTL8139 adapter found"); }
    let next_hop = if destination[0..3] == IP[0..3] { destination } else { GATEWAY };
    let peer = resolve_mac(next_hop, timeout_ticks)?;
    if !send_echo_request(destination, peer, 1) { return Err("could not send ICMP request"); }
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks { if poll_packet(destination, 1) { return Ok(()); } }
    Err("ICMP request timed out")
}

struct TcpPacket {
    sequence: u32,
    acknowledgement: u32,
    flags: u8,
    payload: Vec<u8>,
}

fn tcp_checksum(source: [u8; 4], destination: [u8; 4], segment: &[u8]) -> u16 {
    let mut pseudo = [0u8; 1536];
    let length = 12 + segment.len();
    pseudo[0..4].copy_from_slice(&source);
    pseudo[4..8].copy_from_slice(&destination);
    pseudo[9] = 6;
    pseudo[10..12].copy_from_slice(&(segment.len() as u16).to_be_bytes());
    pseudo[12..length].copy_from_slice(segment);
    checksum(&pseudo[..length])
}

fn send_tcp(
    destination: [u8; 4], destination_mac: [u8; 6], source_port: u16,
    destination_port: u16, sequence: u32, acknowledgement: u32,
    flags: u8, payload: &[u8],
) -> bool {
    let tcp_len = 20 + payload.len();
    let ip_len = 20 + tcp_len;
    let frame_len = 14 + ip_len;
    if frame_len > 1536 { return false; }
    let mut frame = [0u8; 1536];
    frame[0..6].copy_from_slice(&destination_mac);
    frame[6..12].copy_from_slice(&mac_address());
    put16(&mut frame, 12, 0x0800);
    frame[14] = 0x45;
    put16(&mut frame, 16, ip_len as u16);
    put16(&mut frame, 18, timer_get_ticks() as u16);
    frame[22] = 64;
    frame[23] = 6;
    frame[26..30].copy_from_slice(&IP);
    frame[30..34].copy_from_slice(&destination);
    let ip_sum = checksum(&frame[14..34]);
    put16(&mut frame, 24, ip_sum);
    let tcp = 34;
    put16(&mut frame, tcp, source_port);
    put16(&mut frame, tcp + 2, destination_port);
    frame[tcp + 4..tcp + 8].copy_from_slice(&sequence.to_be_bytes());
    frame[tcp + 8..tcp + 12].copy_from_slice(&acknowledgement.to_be_bytes());
    frame[tcp + 12] = 0x50;
    frame[tcp + 13] = flags;
    put16(&mut frame, tcp + 14, 8192);
    frame[tcp + 20..tcp + tcp_len].copy_from_slice(payload);
    let tcp_sum = tcp_checksum(IP, destination, &frame[tcp..tcp + tcp_len]);
    put16(&mut frame, tcp + 16, tcp_sum);
    transmit(&frame[..frame_len])
}

fn poll_tcp(destination: [u8; 4], local_port: u16, remote_port: u16) -> Option<TcpPacket> {
    unsafe {
        let base = IO_BASE;
        if inb(base + 0x37) & 1 != 0 { return None; }
        let ring = core::ptr::addr_of!(RX_BUFFER.0) as *const u8;
        let offset = RX_OFFSET;
        let status = core::ptr::read_unaligned(ring.add(offset) as *const u16);
        let length = core::ptr::read_unaligned(ring.add(offset + 2) as *const u16) as usize;
        let mut result = None;
        if status & 1 != 0 && (54..=RX_CAPACITY).contains(&length) {
            let packet = core::slice::from_raw_parts(ring.add(offset + 4), length);
            if packet[12..14] == 0x0800u16.to_be_bytes() && packet[23] == 6 && packet[30..34] == IP {
                let ihl = (packet[14] as usize & 0x0F) * 4;
                let tcp = 14 + ihl;
                if ihl >= 20 && tcp + 20 <= length && packet[26..30] == destination
                    && u16::from_be_bytes([packet[tcp], packet[tcp + 1]]) == remote_port
                    && u16::from_be_bytes([packet[tcp + 2], packet[tcp + 3]]) == local_port
                {
                    let tcp_len = (packet[tcp + 12] as usize >> 4) * 4;
                    let total_len = u16::from_be_bytes([packet[16], packet[17]]) as usize;
                    if tcp_len >= 20 && ihl + tcp_len <= total_len && 14 + total_len <= length {
                        let payload_start = tcp + tcp_len;
                        let payload_end = 14 + total_len;
                        result = Some(TcpPacket {
                            sequence: u32::from_be_bytes(packet[tcp + 4..tcp + 8].try_into().unwrap()),
                            acknowledgement: u32::from_be_bytes(packet[tcp + 8..tcp + 12].try_into().unwrap()),
                            flags: packet[tcp + 13],
                            payload: packet[payload_start..payload_end].to_vec(),
                        });
                    }
                }
            }
        }
        consume_rx(length);
        result
    }
}

/// Fetch a small plain-HTTP response (port 80). HTTPS/TLS is not included.
pub fn http_get(host: &str, path: &str, timeout_ticks: u32) -> Result<Vec<u8>, &'static str> {
    if !is_available() { return Err("no RTL8139 adapter found"); }
    if !path.starts_with('/') || path.bytes().any(|b| b == b'\r' || b == b'\n')
        || host.bytes().any(|b| b == b'\r' || b == b'\n')
    { return Err("invalid HTTP host or path"); }
    let remote = resolve_ipv4(host, timeout_ticks)?;
    let gateway_mac = resolve_mac(GATEWAY, timeout_ticks)?;
    let source_port = 40000 + (timer_get_ticks() % 20000) as u16;
    let mut local_sequence = (timer_get_ticks() << 16) ^ 0x5057_1337;
    let mut remote_sequence: u32;
    if !send_tcp(remote, gateway_mac, source_port, 80, local_sequence, 0, 0x02, &[]) {
        return Err("could not send TCP SYN");
    }
    let start = timer_get_ticks();
    loop {
        if timer_get_ticks().wrapping_sub(start) >= timeout_ticks { return Err("TCP connection timed out"); }
        if let Some(packet) = poll_tcp(remote, source_port, 80) {
            if packet.flags & 0x04 != 0 { return Err("server refused the TCP connection"); }
            if packet.flags & 0x12 == 0x12 && packet.acknowledgement == local_sequence.wrapping_add(1) {
                remote_sequence = packet.sequence.wrapping_add(1);
                local_sequence = local_sequence.wrapping_add(1);
                if !send_tcp(remote, gateway_mac, source_port, 80, local_sequence, remote_sequence, 0x10, &[]) {
                    return Err("could not acknowledge TCP connection");
                }
                break;
            }
        }
    }

    let request = alloc::format!("GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: PweOS/0.1\r\nConnection: close\r\n\r\n", path, host);
    if !send_tcp(remote, gateway_mac, source_port, 80, local_sequence, remote_sequence, 0x18, request.as_bytes()) {
        return Err("could not send HTTP request");
    }
    local_sequence = local_sequence.wrapping_add(request.len() as u32);
    let mut response = Vec::new();
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks * 3 {
        if let Some(packet) = poll_tcp(remote, source_port, 80) {
            if packet.flags & 0x04 != 0 { return Err("server reset the TCP connection"); }
            if packet.sequence == remote_sequence && !packet.payload.is_empty() {
                remote_sequence = remote_sequence.wrapping_add(packet.payload.len() as u32);
                let available = 4096usize.saturating_sub(response.len());
                response.extend_from_slice(&packet.payload[..packet.payload.len().min(available)]);
                let _ = send_tcp(remote, gateway_mac, source_port, 80, local_sequence, remote_sequence, 0x10, &[]);
            }
            if packet.flags & 0x01 != 0 {
                remote_sequence = remote_sequence.wrapping_add(1);
                let _ = send_tcp(remote, gateway_mac, source_port, 80, local_sequence, remote_sequence, 0x10, &[]);
                break;
            }
            if response.len() >= 4096 { break; }
        }
    }
    if response.is_empty() { Err("HTTP response timed out") } else { Ok(response) }
}
