use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PhysFrame, Size4KiB,
};
use x86_64::{structures::paging::PageTable, PhysAddr, VirtAddr};

/// 有効なレベル4テーブルへの可変参照を返す。
///
/// # Safety
/// 全物理メモリが、渡された
/// `physical_memory_offset`（だけずらしたうえ）で
/// 仮想メモリへとマップされていることを呼び出し元が
/// 保証しなければならない。また、`&mut`参照が複数の
/// 名称を持つこと (mutable aliasingといい、動作が未定義)
/// につながるため、この関数は一度しか呼び出してはならない。
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // ここがunsafe
}

/// 新しいOffsetPageTableを初期化する。
///
/// # Safety
/// 全物理メモリが、渡された
/// `physical_memory_offset`（だけずらしたうえ）で
/// 仮想メモリへとマップされていることを呼び出し元が
/// 保証しなければならない。また、`&mut`参照が複数の
/// 名称を持つこと (mutable aliasingといい、動作が未定義)
/// につながるため、この関数は一度しか呼び出してはならない。
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// ブートローダのメモリマップから、使用可能な
/// フレームを返すFrameAllocator
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// 渡されたメモリマップからFrameAllocatorを作る。
    ///
    /// # Safety
    /// 呼び出し元は渡された
    /// メモリマップが有効であることを保証しなければ
    /// ならない。特に、`USABLE`なフレームは実際に
    /// 未使用でなくてはならない。
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
}

impl BootInfoFrameAllocator {
    /// メモリマップによって指定されたusableなフレームのイテレータを返す。
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // メモリマップからusableな領域を得る
        // 予約済みの領域は除外したい
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // それぞれの領域をアドレス範囲にmapで変換する
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // フレームの開始アドレスのイテレータへと変換する
        // 1Pageが4096B(4KiB)なので、4096ごとになる
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // 開始アドレスから`PhysFrame`型を作る
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
