//! Minimal polling RTL8139 Ethernet driver and IPv4/ARP/ICMP support.
//!
//! The default QEMU machine can use `-netdev user,id=n0 -device rtl8139,netdev=n0`.
//! This intentionally starts with static IPv4 configuration and ICMP echo; it
//! provides a small, debuggable base for later DHCP and socket support.

use crate::arch::io::{inb, inl, inw, outb, outl, outw};
use crate::drivers::timer::timer_get_ticks;

const PCI_ADDR: u16 = 0xCF8;
const PCI_DATA: u16 = 0xCFC;
const RX_RING_SIZE: usize = 8192 + 16 + 1500;
const RX_CAPACITY: usize = 2048;
const TX_CAPACITY: usize = 1536;
const IP: [u8; 4] = [10, 0, 2, 15];
const GATEWAY: [u8; 4] = [10, 0, 2, 2];
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
    if !send_arp_request(next_hop) { return Err("could not send ARP request"); }
    let mut peer = None;
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks {
        unsafe {
            let base = IO_BASE;
            if inb(base + 0x37) & 1 == 0 {
                let offset = RX_OFFSET;
                    let len = core::ptr::read_unaligned((core::ptr::addr_of!(RX_BUFFER.0) as *const u8).add(offset + 2) as *const u16) as usize;
                if len >= 42 && len <= RX_CAPACITY {
                    let p = core::slice::from_raw_parts((core::ptr::addr_of!(RX_BUFFER.0) as *const u8).add(offset + 4), len);
                    if p[12..14] == 0x0806u16.to_be_bytes() && p[20..22] == 2u16.to_be_bytes() && p[28..32] == next_hop {
                        let mut mac = [0; 6]; mac.copy_from_slice(&p[22..28]); peer = Some(mac);
                    }
                }
            }
        }
        if peer.is_some() { break; }
        let _ = poll_packet(destination, 1);
    }
    let peer = peer.ok_or("ARP timed out; check QEMU networking")?;
    if !send_echo_request(destination, peer, 1) { return Err("could not send ICMP request"); }
    let start = timer_get_ticks();
    while timer_get_ticks().wrapping_sub(start) < timeout_ticks { if poll_packet(destination, 1) { return Ok(()); } }
    Err("ICMP request timed out")
}
