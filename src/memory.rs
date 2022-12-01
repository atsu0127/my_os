use x86_64::registers::control::Cr3;
use x86_64::{PhysAddr, structures::paging::PageTable, VirtAddr};

/// 有効なレベル4テーブルへの可変参照を返す。
///
/// # Safety
/// 全物理メモリが、渡された
/// `physical_memory_offset`（だけずらしたうえ）で
/// 仮想メモリへとマップされていることを呼び出し元が
/// 保証しなければならない。また、`&mut`参照が複数の
/// 名称を持つこと (mutable aliasingといい、動作が未定義)
/// につながるため、この関数は一度しか呼び出してはならない。
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // ここがunsafe
}

/// 与えられた仮想アドレスを対応する物理アドレスに変換し、
/// そのアドレスがマップされていないなら`None`を返す。
///
/// # Safety
/// この関数はunsafeである。なぜなら、呼び出し元は全物理メモリが与えられた
/// `physical_memory_offset`（だけずらした上）でマップされていることを
/// 保証しなくてはならないからである。
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// `translate_addr`により呼び出される非公開関数。
///
/// # Safety
/// Rustはunsafeな関数の全体をunsafeブロックとして扱ってしまうので、
/// unsafeの範囲を絞るためにこの関数はunsafeにしていない。
/// この関数をモジュール外から呼び出すときは、
/// unsafeな関数`translate_addr`を使って呼び出すこと。
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // 有効なレベル4フレームをCR3レジスタから読む
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // 複数層のページテーブルを辿る
    for &index in &table_indexes {
        // フレームをページテーブルの参照に変換する
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr}; // ここがunsafe

        // ページテーブルエントリを読んで、`frame`を更新する
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
            //huge pageはサポートしていません
        };
    }

    // ページオフセットを足すことで、目的の物理アドレスを計算する
    Some(frame.start_address() + u64::from(addr.page_offset()))
}