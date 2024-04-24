use sp1_derive::AlignedBorrow;

use crate::air::Block;

#[derive(AlignedBorrow, Default, Debug, Clone)]
#[repr(C)]
pub struct MemoryInitCols<T> {
    pub addr: T,
    pub timestamp: T,
    pub value: Block<T>,
    pub is_real: T,
}

/// NOTE: These are very similar to core/src/memory/columns.rs
/// The reason we cannot use those structs directly is that they use "shard".
/// In our recursive VM, we don't have shards, we only have `clk` (i.e. timestamp).

/// Memory read access.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryReadCols<T> {
    pub access: MemoryAccessCols<T>,
}

/// Memory write access.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryWriteCols<T> {
    pub prev_value: Block<T>,
    pub access: MemoryAccessCols<T>,
}

/// Memory read-write access.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryReadWriteCols<T> {
    pub prev_value: Block<T>,
    pub access: MemoryAccessCols<T>,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryAccessCols<T> {
    /// The value of the memory access.
    pub value: Block<T>,

    /// The previous timestamp that this memory access is being read from.
    pub prev_timestamp: T,

    /// The following columns are decomposed limbs for the difference between the current access's timestamp
    /// and the previous access's timestamp.  Note the actual value of the timestamp is either the
    /// accesses' shard or clk depending on the value of compare_clk.

    /// This column is the least significant 16 bit limb of current access timestamp - prev access timestamp.
    pub diff_16bit_limb: T,

    /// This column is the most signficant 8 bit limb of current access timestamp - prev access timestamp.
    pub diff_8bit_limb: T,
}

/// The common columns for all memory access types.
pub trait MemoryCols<T> {
    fn access(&self) -> &MemoryAccessCols<T>;

    fn access_mut(&mut self) -> &mut MemoryAccessCols<T>;

    fn prev_value(&self) -> &Block<T>;

    fn prev_value_mut(&mut self) -> &mut Block<T>;

    fn value(&self) -> &Block<T>;

    fn value_mut(&mut self) -> &mut Block<T>;
}

impl<T> MemoryCols<T> for MemoryReadCols<T> {
    fn access(&self) -> &MemoryAccessCols<T> {
        &self.access
    }

    fn access_mut(&mut self) -> &mut MemoryAccessCols<T> {
        &mut self.access
    }

    fn prev_value(&self) -> &Block<T> {
        &self.access.value
    }

    fn prev_value_mut(&mut self) -> &mut Block<T> {
        &mut self.access.value
    }

    fn value(&self) -> &Block<T> {
        &self.access.value
    }

    fn value_mut(&mut self) -> &mut Block<T> {
        &mut self.access.value
    }
}

impl<T> MemoryCols<T> for MemoryWriteCols<T> {
    fn access(&self) -> &MemoryAccessCols<T> {
        &self.access
    }

    fn access_mut(&mut self) -> &mut MemoryAccessCols<T> {
        &mut self.access
    }

    fn prev_value(&self) -> &Block<T> {
        &self.prev_value
    }

    fn prev_value_mut(&mut self) -> &mut Block<T> {
        &mut self.prev_value
    }

    fn value(&self) -> &Block<T> {
        &self.access.value
    }

    fn value_mut(&mut self) -> &mut Block<T> {
        &mut self.access.value
    }
}

impl<T> MemoryCols<T> for MemoryReadWriteCols<T> {
    fn access(&self) -> &MemoryAccessCols<T> {
        &self.access
    }

    fn access_mut(&mut self) -> &mut MemoryAccessCols<T> {
        &mut self.access
    }

    fn prev_value(&self) -> &Block<T> {
        &self.prev_value
    }

    fn prev_value_mut(&mut self) -> &mut Block<T> {
        &mut self.prev_value
    }

    fn value(&self) -> &Block<T> {
        &self.access.value
    }

    fn value_mut(&mut self) -> &mut Block<T> {
        &mut self.access.value
    }
}
