#[cfg(feature = "alloc")]
pub use alloc::collections::*;

// 导出hashbrown库的HashMap
#[cfg(feature = "alloc")]
pub use hashbrown::HashMap;
