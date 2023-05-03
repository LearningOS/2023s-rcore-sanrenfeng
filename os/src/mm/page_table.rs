//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum, PhysAddr};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
    /// 翻译给定的虚拟地址，获得物理地址
   pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

  /// start 没有按页大小对齐
  /// port & !0x7 != 0 (port 其余位必须为0)
  /// port & 0x7 = 0 (这样的内存无意义)
  /// [start, start + len) 中存在已经被映射的页
  /// 物理内存不足
  /// mmap
   pub fn mmap(&mut self,start:usize,len:usize,port:usize)->isize{
        let start_va : VirtAddr = start.into();
        let end_va : VirtAddr = (start + len).into();
        let start_vpn : VirtPageNum= start_va.floor().into();
        let end_vpn : VirtPageNum= end_va.ceil().into();

        info!("port is {}",port);
        if start_va.page_offset()!=0{
            return -1;
        }
        if (port & !0x7 !=0 ) || (port & 0x7 ==0) {
            return -1;
        }
        // 第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
        let mut flag = PTEFlags::U | PTEFlags::V;
        if port & 1 == 1{
            flag = flag | PTEFlags::R;
        } 
        if port>>1 & 1 == 1{
            flag = flag | PTEFlags::W;
        }
        if port>>2 & 1 == 1{
            flag = flag | PTEFlags::X;
        }
        info!("{:#?} =>> {:#?} =>> {:#?} =>> {:#?}",start_va,end_va,start_vpn,end_vpn);
        for num in start_vpn.0..=end_vpn.0{
             let vpn  = num.into();
             let pte = self.find_pte_create(vpn).unwrap();
             if pte.is_valid(){
                 info!("this is has map  ========");
                 return -1;
             }
             info!("maping {:#?} {:#?} {:#?} =>>>> {:#?}",start_vpn,end_vpn,vpn,flag);
             *pte = PageTableEntry::new(vpn.0.into(),flag);
         }
        0
   }
   /// unmmap 
    pub fn unmmap(&mut self,start:usize,len:usize)->isize{
        let start_va : VirtAddr = start.into();
        let end_va : VirtAddr = (start + len).into();
        let start_vpn : VirtPageNum= start_va.floor().into();
        let end_vpn : VirtPageNum= end_va.ceil().into();
        for num in start_vpn.0..=end_vpn.0{
             let vpn  = num.into();
             match self.find_pte(vpn){
                None => return -1,
                Some(pte) =>{
                    if !pte.is_valid(){
                       return -1;
                  }
                *pte = PageTableEntry::empty();

                }
             }
                      }
        0
    

    }

}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
/// va =>>> pa
pub fn translate_va_pa(token: usize,va : VirtAddr) -> Option<PhysAddr>{
    PageTable::from_token(token).translate_va(va.clone())
}

/// 根据va 获得对于的类型T
pub fn translate_va_t<T>(token:usize,va:VirtAddr) -> &'static mut T{
    translate_va_pa(token,va).unwrap().get_mut()
}

/// mmap
pub fn mmap(token:usize,start:usize,len:usize,port:usize)->isize{
    PageTable::from_token(token).mmap(start, len, port)
}



/// unmmap
pub fn unmmap(token:usize,start:usize,len:usize)->isize{
    PageTable::from_token(token).unmmap(start, len)
}
