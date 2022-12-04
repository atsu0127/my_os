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

/// 与えられたページを物理フレーム`0xb8000`にマップする
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    // PRESENT: 有効なエントリに必須のフラグ
    // WRITABLE: ページを書き込み可能にする
    let flags = Flags::PRESENT | Flags::WRITABLE;

    // 呼び出し元のフレームがまだ使われてないことを保証しないとなのでunsafe
    let map_to_result = unsafe {
        // FIXME: unsafeで、テスト目的なので後で取り除く
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    // flush: マッピングしたページをTLBからflushできる
    map_to_result.expect("map_to failed").flush();
}

/// 常に`None`を返すFrameAllocator
pub struct EmptyFrameAllocator;

// 実装したアロケーたが未使用のフレームのみを取得することを保証しないといけないのでunsafe
unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}
