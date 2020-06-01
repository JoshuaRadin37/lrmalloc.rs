use std::mem::size_of;

pub const MAX_SZ_IDX: usize = 40usize;
pub const LG_MAX_SIZE_IDX: usize = 6_usize;
pub const MAX_SZ: usize = (1 << 13) + (1 << 11) * 3;
pub const LG_PTR: usize = size_of::<*const usize>();
/// cache line is 64 bytes
pub const LG_CACHE_LINE: usize = 6;
/// a Page is is 4kb
pub const LG_PAGE: usize = 12;
/// a huge page is 2mb
pub const LG_HUGE_PAGE: usize = 21;

pub const PTR_SIZE: usize = 1usize << LG_PTR;
pub const CACHE_LINE: usize = 1usize << LG_CACHE_LINE;
pub const PAGE: usize = 1usize << LG_PAGE;
pub const HUGE_PAGE: usize = 1 << LG_HUGE_PAGE;

pub const PTR_MASK: usize = PTR_SIZE - 1;
pub const CACHE_LINE_MASK: usize = CACHE_LINE - 1;
pub const PAGE_MASK: usize = (PAGE - 1);

pub const MIN_ALIGN: usize = LG_PTR;