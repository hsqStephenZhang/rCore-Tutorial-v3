//! physical and virtual address types

use core::fmt;
use core::fmt::Debug;
use core::ops::Add;

use crate::config::*;

use super::page_table::PageTableEntry;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);
impl PhysPageNum {
    pub unsafe fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    pub unsafe  fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    pub unsafe fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

impl VirtPageNum {
    /// ret[0]: [30, 38], ret[1]: [21, 29], ret[2]: [12, 20]
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut indexes = [0; 3];
        for i in (0..3).rev() {
            indexes[i] = vpn & ((1 << 9) - 1);
            vpn >>= 9;
        }
        indexes
    }
}

impl Add<usize> for PhysPageNum {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<usize> for VirtPageNum {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

impl PhysAddr {
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
    pub fn offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
}

impl VirtAddr {
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
}

impl From<usize> for PhysAddr {
    fn from(addr: usize) -> Self {
        PhysAddr(addr & ((1 << PA_WIDTH_SV39) - 1))
    }
}

impl From<usize> for PhysPageNum {
    fn from(addr: usize) -> Self {
        PhysPageNum(addr & ((1 << PPN_WIDTH_SV39) - 1))
    }
}

impl From<usize> for VirtAddr {
    fn from(addr: usize) -> Self {
        VirtAddr(addr & ((1 << VA_WIDTH_SV39) - 1))
    }
}

impl From<usize> for VirtPageNum {
    fn from(addr: usize) -> Self {
        VirtPageNum(addr & ((1 << VPN_WIDTH_SV39) - 1))
    }
}

impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

impl From<PhysPageNum> for usize {
    fn from(addr: PhysPageNum) -> Self {
        addr.0
    }
}

impl From<VirtAddr> for usize {
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}

impl From<VirtPageNum> for usize {
    fn from(addr: VirtPageNum) -> Self {
        addr.0
    }
}

impl From<PhysAddr> for PhysPageNum {
    fn from(addr: PhysAddr) -> Self {
        addr.floor()
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(ppn: PhysPageNum) -> Self {
        PhysAddr(ppn.0 << PAGE_SIZE_BITS)
    }
}

impl From<VirtAddr> for VirtPageNum {
    fn from(addr: VirtAddr) -> Self {
        addr.floor()
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(vpn: VirtPageNum) -> Self {
        VirtAddr(vpn.0 << PAGE_SIZE_BITS)
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PA: (0x{:x})", self.0)
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VA: (0x{:x})", self.0)
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PPN: (0x{:x})", self.0)
    }
}

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VPN: (0x{:x})", self.0)
    }
}


pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Debug, Copy, Clone)]
/// a simple range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

/// a simple range structure for virtual page number
pub type VPNRange = SimpleRange<VirtPageNum>;

impl VPNRange{
    pub fn from_addr_range(start: VirtAddr, end: VirtAddr) -> Self {
        VPNRange::new(start.floor(), end.ceil())
    }
}

pub fn addr_test() {
    let pa = PhysAddr(0x1234);
    let va = VirtAddr(0x5678);
    let ppn = PhysPageNum(0x9);
    let vpn = VirtPageNum(0xA);

    assert_eq!(pa.floor(), PhysPageNum(0x1));
    assert_eq!(pa.ceil(), PhysPageNum(0x2));
    assert_eq!(pa.offset(), 0x1234 & (PAGE_SIZE - 1));

    assert_eq!(va.floor(), VirtPageNum(0x5));
    assert_eq!(va.ceil(), VirtPageNum(0x6));
    assert_eq!(va.page_offset(), 0x5678 & (PAGE_SIZE - 1));

    assert_eq!(
        PhysAddr::from(0x1234),
        PhysAddr(0x1234 & ((1 << PA_WIDTH_SV39) - 1))
    );
    assert_eq!(
        PhysPageNum::from(0x9),
        PhysPageNum(0x9 & ((1 << PPN_WIDTH_SV39) - 1))
    );
    assert_eq!(
        VirtAddr::from(0x5678),
        VirtAddr(0x5678 & ((1 << VA_WIDTH_SV39) - 1))
    );
    assert_eq!(
        VirtPageNum::from(0xA),
        VirtPageNum(0xA & ((1 << VPN_WIDTH_SV39) - 1))
    );

    assert_eq!(usize::from(pa), 0x1234);
    assert_eq!(usize::from(ppn), 0x9);
    assert_eq!(usize::from(va), 0x5678);
    assert_eq!(usize::from(vpn), 0xA);

    assert_eq!(PhysPageNum::from(pa), PhysPageNum(0x1));
    assert_eq!(PhysAddr::from(ppn), PhysAddr(0x9 << PAGE_SIZE_BITS));
    assert_eq!(VirtPageNum::from(va), VirtPageNum(0x5));
    assert_eq!(VirtAddr::from(vpn), VirtAddr(0xA << PAGE_SIZE_BITS));

    let vpn = VirtPageNum(0b000000001_000000011_000000111);
    let indexes = vpn.indexes();
    assert!(indexes == [0b1, 0b11, 0b111]);

    println!("address test passed!");
}
