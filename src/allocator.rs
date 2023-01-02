use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

/// mapperとframe_allocatorを引数にとる
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    //
    let page_range = {
        // Heapの開始アドレスを仮想アドレスに変更
        let heap_start = VirtAddr::new(HEAP_START as u64);
        // Heapの終端アドレス(端が含まれてほしいので1を引く)
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        // 指定したページ範囲の作成
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        // ページのマップされるフレームを取得する
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        // Flag準備
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        // ページテーブルに新たに対応を作る
        // 最後にflushを呼ぶことでTLB(変換内容のキャッシュ)を更新する
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // Allocatorの初期化
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
