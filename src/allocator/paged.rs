use std::alloc::Layout;
use std::ptr::NonNull;

/// A frame allocator that uses a chain of pages to support unlimited growth.
///
/// Unlike `FrameAllocator` which is fixed-size, `PagedFrameAllocator` allocates
/// new pages from the heap as needed. Pages are cached and reused across frames,
/// preventing repeated malloc/free overhead once steady state is reached.
pub struct PagedFrameAllocator {
    /// Pages of memory.
    /// The first `active_pages` are in use for the current frame.
    /// The rest are cached for reuse.
    pages: Vec<Box<[u8]>>,
    
    /// Index of the current page being written to.
    current_page_index: usize,
    
    /// Offset in the current page.
    offset: usize,
    
    /// Size of each page in bytes.
    page_size: usize,
}

impl PagedFrameAllocator {
    /// Creates a new paged allocator.
    pub fn new(page_size: usize) -> Self {
        // Ensure reasonable page size (min 64KB)
        let page_size = page_size.max(64 * 1024);
        
        let mut allocator = Self {
            pages: Vec::new(),
            current_page_index: 0,
            offset: 0,
            page_size,
        };
        
        // Pre-allocate one page
        allocator.add_page();
        allocator
    }
    
    /// Adds a new page to the pool.
    fn add_page(&mut self) {
        // We use Box<[u8]> to ensure stable address for the page content,
        // although the vector resizing might move the Box pointers themselves? 
        // No, Vec<Box<T>>: the Boxes are on heap, moving the Box struct is fine, 
        // the pointer strictly inside the Box points to the heap buffer which is stable.
        self.pages.push(vec![0u8; self.page_size].into_boxed_slice());
    }

    /// Allocates memory with the given layout.
    pub fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        // 1. Try to align in current page
        let current_ptr = self.pages[self.current_page_index].as_ptr() as usize;
        let mut aligned_offset = (self.offset + align - 1) & !(align - 1);

        // Check if fits in current page
        if aligned_offset + size > self.page_size {
            // Does not fit. 
            // If the requested allocation is larger than a whole page, we are in trouble.
            // For now, we assume jobs are small. If a single alloc > page_size, we fall back to heap (return None).
            if size > self.page_size {
                return None; 
            }

            // Move to next page
            self.current_page_index += 1;
            
            // If we don't have a next page, create one
            if self.current_page_index >= self.pages.len() {
                self.add_page();
            }
            
            // Reset offset for new page
            self.offset = 0;
            aligned_offset = 0; // First byte is always aligned to page alignment (usually high enough)
             
            // Verify alignment of page start (Box allocation usually aligned to max align)
             let new_page_ptr = self.pages[self.current_page_index].as_ptr() as usize;
             let new_aligned_offset = (new_page_ptr + align - 1) & !(align - 1);
             let padding = new_aligned_offset - new_page_ptr;
             aligned_offset = padding;
        }

        // We have a page and offset that fits
        let page_base = self.pages[self.current_page_index].as_mut_ptr();
        let ptr = unsafe { page_base.add(aligned_offset) };
        
        self.offset = aligned_offset + size;

        NonNull::new(ptr)
    }

    /// Resets the allocator for a new frame.
    ///
    /// This keeps all pages allocated but resets pointers to the beginning.
    /// This is where the "Zero-Overhead" comes from in steady state.
    pub fn reset(&mut self) {
        self.current_page_index = 0;
        self.offset = 0;
    }
    
    /// Returns the total capacity allocated in bytes (resident set size approximated).
    pub fn total_capacity(&self) -> usize {
        self.pages.len() * self.page_size
    }
}
