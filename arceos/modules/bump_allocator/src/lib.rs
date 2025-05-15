#![no_std]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize = 4096> {
    // 内存区域起始地址
    start: usize,
    // 内存区域结束地址
    end: usize,
    // 字节分配当前位置
    byte_pos: usize,
    // 页分配当前位置
    page_pos: usize,
    // 字节分配计数器
    alloc_count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    /// 创建一个新的早期内存分配器
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            byte_pos: 0,
            page_pos: 0,
            alloc_count: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Initialize the allocator with a free memory region.
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.byte_pos = start;
        self.page_pos = self.end;
        self.alloc_count = 0;
    }

    /// Add a free memory region to the allocator.
    fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
        // 早期分配器不支持添加新的内存区域
        Err(AllocError::NoMemory)
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Allocate memory with the given size (in bytes) and alignment.
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        if size == 0 {
            return Err(AllocError::InvalidParam);
        }

        // 计算对齐后的位置
        let aligned_pos = (self.byte_pos + align - 1) & !(align - 1);
        let new_pos = aligned_pos + size;

        // 检查是否有足够空间
        if new_pos > self.page_pos {
            return Err(AllocError::NoMemory);
        }

        // 更新分配计数和位置指针
        self.byte_pos = new_pos;
        self.alloc_count += 1;

        // 返回分配的内存区域
        NonNull::new(aligned_pos as *mut u8).ok_or(AllocError::InvalidParam)
    }

    /// Deallocate memory at the given position, size, and alignment.
    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        // 减少分配计数
        if self.alloc_count > 0 {
            self.alloc_count -= 1;
        }

        // 只有当所有分配都释放时，才重置字节分配指针
        if self.alloc_count == 0 {
            self.byte_pos = self.start;
        }
        // 注意：我们不会释放单个内存块，而是等到所有块都释放时才重置指针
    }

    /// Returns total memory size in bytes.
    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    /// Returns allocated memory size in bytes.
    fn used_bytes(&self) -> usize {
        let bytes_used = self.byte_pos - self.start;
        let pages_used = self.end - self.page_pos;
        bytes_used + pages_used
    }

    /// Returns available memory size in bytes.
    fn available_bytes(&self) -> usize {
        self.page_pos - self.byte_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;
    /// Allocate contiguous memory pages with given count and alignment.
    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if num_pages == 0 {
            return Err(AllocError::InvalidParam);
        }

        let page_size = PAGE_SIZE;
        let bytes_size = num_pages * page_size;

        // 计算对齐要求，align_pow2表示页对齐的幂，例如0表示1页对齐，1表示2页对齐...
        let align = page_size << align_pow2;

        // 计算对齐后的页起始位置（向下对齐）
        let aligned_pos = (self.page_pos - bytes_size) & !(align - 1);

        // 检查是否有足够空间
        if aligned_pos < self.byte_pos {
            return Err(AllocError::NoMemory);
        }

        // 更新页分配位置
        self.page_pos = aligned_pos;

        // 返回分配的页地址
        Ok(aligned_pos)
    }

    /// Deallocate contiguous memory pages with given position and count.
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // 页分配不支持释放，这是设计决定
        // 早期分配器中分配的页面预计会在整个系统生命周期内使用
    }

    /// Returns the total number of memory pages.
    fn total_pages(&self) -> usize {
        self.total_bytes() / Self::PAGE_SIZE
    }

    /// Returns the number of allocated memory pages.
    fn used_pages(&self) -> usize {
        (self.end - self.page_pos) / Self::PAGE_SIZE
    }

    /// Returns the number of available memory pages.
    fn available_pages(&self) -> usize {
        (self.page_pos - self.byte_pos) / Self::PAGE_SIZE
    }
}
