//! Helper struct to build a `Function`.
use device::Device;
use ir::{AccessPattern, Function, Signature, InstId, Operand, Operator};
use ir::{self, Parameter, Size, Type, dim, mem, op};
use itertools::Itertools;
use helper::{AutoOperand, LogicalDim, MetaDimension, MetaBasicBlock};
use search_space::{Action, Order, DimKind, InstFlag, MemSpace, SearchSpace};
use std::borrow::Borrow;
use utils::*;

/// Helper to build a `Function`.
pub struct Builder<'a> {
    function: Function<'a>,
    open_dims: HashMap<dim::Id, dim::Id>,
    actions: Vec<Action>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` for a `Function` with the given signature.
    pub fn new(signature: &'a Signature, device: &'a Device) -> Builder<'a> {
        Builder {
            function: Function::new(signature, device),
            open_dims: HashMap::default(),
            actions: Vec::new(),
        }
    }

    /// Returns the function created by the builder
    pub fn get(self) -> SearchSpace<'a> {
        debug!("{:?}", self.actions);
        SearchSpace::new(self.function, self.actions).expect("invalid IR instance")
    }

    /// Returns the function created by the builder
    pub fn get_clone(&self) -> SearchSpace<'a> {
        let function = self.function.clone();
        SearchSpace::new(function, self.actions.clone()).expect("invalid IR instance")
    }

    /// Returns an operand from an `AutoOperand`.
    fn get_op<'b: 'a>(&self, op: &AutoOperand<'b>) -> Operand<'a> {
        op.get(&self.function, &self.open_dims)
    }

    /// Returns the size of a dimension.
    pub fn dim_size(&self, dim: ir::dim::Id) -> &ir::Size<'a> {
        self.function.dim(dim).size()
    }

    /// Creates a binary operator.
    pub fn binop<'b: 'a>(&mut self,
                         op: ir::BinOp,
                         lhs: &AutoOperand<'b>,
                         rhs: &AutoOperand<'b>) -> InstId {
        let lhs_op = self.get_op(lhs);
        let rhs_op = self.get_op(rhs);
        let rounding = default_rounding(&lhs_op.t());
        self.inst(op::BinOp(op, lhs_op, rhs_op, rounding))
    }

    /// Adds an `Add` instruction to the fuction.
    pub fn add<'b: 'a>(&mut self, lhs: &AutoOperand<'b>, rhs: &AutoOperand<'b>)
        -> InstId
    {
        self.binop(ir::BinOp::Add, lhs, rhs)
    }

    /// Adds a `Sub` instruction to the function.
    pub fn sub<'b: 'a>(&mut self, lhs: &AutoOperand<'b>, rhs: &AutoOperand<'b>
                      ) -> InstId {
        self.binop(ir::BinOp::Sub, lhs, rhs)
    }

    /// Adds a `Mul` instruction to the function. Defaults to low mode.
    pub fn mul<'b: 'a>(&mut self, lhs: &AutoOperand<'b>, rhs: &AutoOperand<'b>
                      ) -> InstId {
        let lhs_op = self.get_op(lhs);
        let rhs_op = self.get_op(rhs);
        let t = lhs_op.t();
        let rounding = default_rounding(&t);
        self.inst(op::Mul(lhs_op, rhs_op, rounding, t))
    }

    /// Adds a 'Mul` instruction with a wide mode to the function.
    pub fn mul_ex<'b: 'a>(&mut self, lhs: &AutoOperand<'b>, rhs: &AutoOperand<'b>,
                          t: Type) -> InstId {
        let lhs_op = self.get_op(lhs);
        let rhs_op = self.get_op(rhs);
        let rounding = default_rounding(&t);
        let op = op::Mul(lhs_op, rhs_op, rounding, t);
        self.inst(op)
    }

    /// Adds a `Mad` or `Fma` instruction to the function. Defaults to low or wide mode
    /// depending on the operand types.
    pub fn mad<'b: 'a>(&mut self, mul_lhs: &AutoOperand<'b>, mul_rhs: &AutoOperand<'b>,
                       add_rhs: &AutoOperand<'b>) -> InstId {
        let mul_lhs_op = self.get_op(mul_lhs);
        let mul_rhs_op = self.get_op(mul_rhs);
        let add_rhs_op = self.get_op(add_rhs);
        let rounding = default_rounding(&mul_lhs_op.t());
        let op = op::Mad(mul_lhs_op, mul_rhs_op, add_rhs_op, rounding);
        self.inst(op)
    }

    /// Adds a `Div` instruction to the fuction.
    pub fn div<'b: 'a>(&mut self, lhs: &AutoOperand<'b>, rhs: &AutoOperand<'b>
                      ) -> InstId {
        self.binop(ir::BinOp::Div, lhs, rhs)
    }

    /// Adds a `Mov` instruction to the function.
    pub fn mov<'b: 'a>(&mut self, arg: &AutoOperand<'b>) -> InstId {
        let arg_op = self.get_op(arg);
        self.inst(op::Mov(arg_op))
    }

    /// Adds a coherent load from global memory instruction to the function.
    pub fn ld<'b: 'a>(&mut self, ret_type: Type, addr: &AutoOperand<'b>,
                      pattern: AccessPattern<'a>) -> InstId {
        self.ld_ex(ret_type, addr, pattern, InstFlag::MEM_COHERENT)
    }

    /// Adds a non-coherent load from global memory instruction to the function.
    pub fn ld_nc<'b: 'a>(&mut self, ret_type: Type, addr: &AutoOperand<'b>,
                        pattern: AccessPattern<'a>) -> InstId {
        self.ld_ex(ret_type, addr, pattern, InstFlag::ALL)
    }

    /// Adds a load instruction with the given flags and memory block.
    pub fn ld_ex<'b: 'a>(&mut self,
                         ret_type: Type,
                         addr: &AutoOperand<'b>,
                         pattern: AccessPattern<'a>,
                         flags: InstFlag) -> InstId {
        let addr_op = self.get_op(addr);
        let inst_id = self.inst(op::Ld(ret_type, addr_op, pattern));
        self.actions.push(Action::InstFlag(inst_id, flags));
        inst_id
    }

    /// Adds a store instruction.
    pub fn st<'b: 'a>(&mut self, addr: &AutoOperand<'b>, val: &AutoOperand<'b>,
                      pattern: AccessPattern<'a>) -> InstId {
        self.st_ex(addr, val, true, pattern, InstFlag::ALL)
    }

    /// Adds a store instruction with the given flags and memory block.
    pub fn st_ex<'b :'a>(&mut self,
                         addr: &AutoOperand<'b>,
                         val: &AutoOperand<'b>,
                         side_effect: bool,
                         pattern: AccessPattern<'a>,
                         flags: InstFlag,
                        ) -> InstId {
        let addr_op = self.get_op(addr);
        let val_op = self.get_op(val);
        let inst_id = self.inst(op::St(addr_op, val_op, side_effect, pattern));
        self.actions.push(Action::InstFlag(inst_id, flags));
        inst_id
    }

    /// Adds a cast instruction to the given type.
    pub fn cast<'b: 'a>(&mut self, val: &AutoOperand<'b>, t: Type) -> InstId {
        let val_op = self.get_op(val);
        self.inst(op::Cast(val_op, t))
    }

    /// Returns the type of an operand.
    pub fn type_of<'b: 'a>(&self, op: &AutoOperand<'b>) -> ir::Type {
        self.get_op(op).t()
    }

    /// Restricts the order between two basic blocks. Does not restricts LINK and NPACK
    /// flags.
    pub fn order(&mut self, lhs: &MetaBasicBlock, rhs: &MetaBasicBlock, order: Order) {
        for lhs in lhs.borrow().ids() {
            for rhs in rhs.borrow().ids() {
                self.action(Action::Order(lhs, rhs, order));
            }
        }
    }

    /// Register that two dimensions are mapped.
    pub fn map_dims(&mut self, lhs: &MetaDimension, rhs: &MetaDimension) {
        for (lhs, rhs) in lhs.ids().zip_eq(rhs.ids()) {
            let should_map = {
                let lhs_dim = self.function.dim(lhs);
                lhs_dim.possible_sizes().is_some() && !lhs_dim.is_mapped_dim(rhs)
            };
            if should_map { self.function.set_dims_mapped(lhs, rhs); }
        }
    }

    /// Inserts an instruction in the function.
    fn inst(&mut self, op: Operator<'a>) -> InstId {
        self.function.add_inst(op, self.open_dims.iter().map(|(&x, _)| x).collect())
    }

    /// Builds both an induction variable for a tensor memory access and the corresponding
    /// access pattern.
    pub fn tensor_access(&mut self, addr: &AutoOperand<'a>,
                         mem: ir::mem::Id,
                         t: &ir::Type,
                         dims: &[&MetaDimension])
        -> (ir::IndVarId, ir::AccessPattern<'a>)
    {
        let data_size = self.cst_size(unwrap!(t.len_byte()));
        let induction_dims = dims.iter().flat_map(|d| d.ids()).rev()
            .scan(data_size, |size, dim| {
                let increment = size.clone();
                *size *= self.function.dim(dim).size();
                Some((dim, increment))
            }).collect();
        let index = self.induction_var(addr, induction_dims);
        (index, self.tensor_access_pattern(mem, t, dims))
    }

    /// Applies an action on the function.
    pub fn action(&mut self, action: Action) { self.actions.push(action) }

    /// Opens a new dimension.
    pub fn open_dim(&mut self, size: Size<'a>) -> dim::Id {
        let id = self.function.add_dim(size);
        self.open_dims.insert(id, id);
        id
    }

    /// Opens a nest of new dimension with the given kinds and sizes.
    pub fn open_dim_ex(&mut self, size: Size<'a>, kind: DimKind) -> dim::Id {
        let id = self.open_dim(size);
        self.actions.push(Action::DimKind(id, kind));
        id
    }

    /// Open multiple dimensions to represent a tiled dimension.
    pub fn open_tiled_dim(&mut self, size: Size<'a>,
                          tiling_factors: Vec<u32>,
                          num_tile_dims: usize) -> LogicalDim<'a> {
        if num_tile_dims == 0 {
            LogicalDim::Simple(self.open_dim(size))
        } else {
            let (id, dims) = self.function.add_logical_dim(
                size.clone(), tiling_factors.clone(), num_tile_dims);
            self.open_dims.extend(dims.iter().map(|&id| (id, id)));
            LogicalDim::Composite { id, dims, size, tiling_factors }
        }
    }

    /// Opens a new dimension mapped to an existing one.
    ///
    /// The size of the new dim is inherited from the mapped dim.
    /// The dimension mapped to is closed if needed.
    pub fn open_mapped_dim(&mut self, old_dim: &LogicalDim<'a>) -> LogicalDim<'a> {
        let new_dim = match old_dim {
            LogicalDim::Simple(old_id) => {
                self.open_dims.remove(old_id);
                let size = {
                    let dim = self.function.dim(*old_id);
                    if let Some(&[size]) = dim.possible_sizes() {
                        ir::Size::new_const(size)
                    } else {
                        dim.size().clone()
                    }
                };
                let new_id = self.function.add_dim(size);
                self.open_dims.insert(new_id, *old_id);
                LogicalDim::Simple(new_id)
            },
            LogicalDim::Composite { dims: old_dims, size, tiling_factors, .. } => {
                let (new_id, new_dims) = self.function.add_logical_dim(
                    size.clone(), tiling_factors.clone(), old_dims.len() - 1);
                for (&old, &new) in old_dims.iter().zip_eq(&new_dims) {
                    self.open_dims.remove(&old);
                    self.open_dims.insert(new, old);
                }
                LogicalDim::Composite {
                    id: new_id,
                    dims: new_dims,
                    size: size.clone(),
                    tiling_factors: tiling_factors.clone(),
                }
            },
        };
        for (old, new) in
            MetaDimension::ids(old_dim).zip_eq(MetaDimension::ids(&new_dim))
        {
            if self.function.dim(old).possible_sizes().is_some() {
                self.function.set_dims_mapped(old, new);
            }
        }
        new_dim
    }

    /// Opens an existing dimension.
    pub fn reopen_dim(&mut self, dim: &MetaDimension) {
        for id in dim.ids() { self.open_dims.insert(id, id); }
    }

    /// Opens an existing dimension and maps it to another one.
    /// The dimension mapped to is closed if needed.
    pub fn reopen_mapped_dim(&mut self, dim: &MetaDimension, mapped_to: &MetaDimension) {
        for (dim, mapped_to) in dim.ids().zip_eq(mapped_to.ids()) {
            self.open_dims.remove(&mapped_to);
            self.open_dims.insert(dim, mapped_to);
        }
    }

    /// Closes a dimension.
    pub fn close_dim(&mut self, dims: &MetaDimension) {
        for dim in dims.ids() { assert!(self.open_dims.remove(&dim).is_some()); }
    }

    /// Returns a constant size.
    pub fn cst_size(&self, size: u32) -> Size<'a> { Size::new_const(size) }

    /// Returns a parameter size.
    pub fn param_size(&self, param: &str) -> Size<'a> {
        Size::new_param(self.find_param(param))
    }

    /// Allocates a memory block in shared memory.
    pub fn allocate_shared(&mut self, size: u32) -> mem::InternalId {
        let id = self.allocate(size, true);
        self.actions.push(Action::MemSpace(id.into(), MemSpace::SHARED));
        id
    }

    /// Allocates a memory block.
    pub fn allocate(&mut self, size: u32, private: bool) -> mem::InternalId {
        self.function.add_mem_block(size, private)
    }

    /// Generates an access paterns with all the strides unknown on the opened dimensions.
    pub fn unknown_access_pattern(&self, mem: mem::Id) -> AccessPattern<'static> {
        AccessPattern::Unknown { mem_id: mem }
    }

    /// Generates the access pattern corresponding to accessing a tensor of the given
    /// type. The data is assumed to be laid out contiguously in the order given by
    /// dimensions. The last dimension is the major order.
    pub fn tensor_access_pattern(&self, mem: mem::Id, t: &Type, dims: &[&MetaDimension])
            -> AccessPattern<'a> {
        let data_size = self.cst_size(unwrap!(t.len_byte()));
        let dims = dims.iter().flat_map(|d| d.ids()).rev().scan(data_size, |size, dim| {
            let increment = size.clone();
            *size *= self.function.dim(dim).size();
            Some((dim, increment))
        }).collect();
        AccessPattern::Tensor { mem_id: mem, dims }
    }

    /// Builds an induction variable.
    pub fn induction_var(&mut self, base: &AutoOperand<'a>,
                         dims: Vec<(dim::Id, ir::Size<'a>)>) -> ir::IndVarId {
        let base = self.get_op(base);
        self.function.add_ind_var(ir::InductionVar::new(dims, base))
    }

    /// Creates a dim-map operand.
    pub fn dim_map(&mut self, base: ir::InstId,
                   dim_map: &[(&MetaDimension, &MetaDimension)],
                   scope: ir::DimMapScope) -> ir::Operand<'a> {
        let dim_map = ir::DimMap::new(dim_map.iter().flat_map(|&(lhs, rhs)| {
            self.map_dims(lhs, rhs);
            lhs.ids().zip_eq(rhs.ids())
        }));
        let inst = self.function.inst(base);
        ir::Operand::new_inst(inst, dim_map, scope, &self.function)
    }

    /// Finds a paramter given its name.
    pub fn find_param(&self, param: &str) -> &'a Parameter {
        unwrap!(self.function.signature().params.iter().find(|p| p.name == param))
    }
}

/// Returns the default rounding for a given operand type.
fn default_rounding(t: &Type) -> op::Rounding {
    if t.is_integer() { op::Rounding::Exact } else { op::Rounding::Nearest }
}
