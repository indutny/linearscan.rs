// Private imports
use linearscan::graph::{Block, Instruction, User, Phi, ToPhi};

// Public API
pub use linearscan::graph::{Graph,
                            UseKind, UseAny, UseRegister, UseFixed,
                            BlockId, InstrId, StackId,
                            Value, RegisterVal, StackVal};
pub use linearscan::allocator::{Allocator, Config};
pub use linearscan::generator::{Generator, GeneratorFunctions};

struct BlockBuilder<'self, K, G, R> {
  graph: &'self mut Graph<K, G, R>,
  block: BlockId
}

pub trait GroupHelper: Clone+Eq {
  fn any() -> Self;
  fn to_uint(&self) -> uint;
  fn from_uint(i: uint) -> Self;
}

pub trait RegisterHelper<Group>: Clone+Eq {
  fn group(&self) -> Group;
  fn to_uint(&self) -> uint;
  fn from_uint(g: &Group, i: uint) -> Self;
}

pub trait KindHelper<G: GroupHelper, R: RegisterHelper<G> >: Clone {
  fn clobbers(&self, group: &G) -> bool;
  fn temporary(&self) -> ~[G];
  fn use_kind(&self, i: uint) -> UseKind<G, R>;
  fn result_kind(&self) -> Option<UseKind<G, R> >;
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R> > Graph<K, G, R> {
  /// Create empty block
  pub fn empty_block(&mut self) -> BlockId {
    let block = ~Block::new(self);
    let id = block.id;
    self.blocks.insert(id.to_uint(), block);
    return id;
  }

  /// Create empty block and initialize it in the block
  pub fn block(&mut self, body: &fn(b: &mut BlockBuilder<K, G, R>)) -> BlockId {
    let block = ~Block::new(self);
    let id = block.id;
    self.blocks.insert(id.to_uint(), block);

    // Execute body
    self.with_block(id, body);

    return id;
  }

  /// Create phi value
  pub fn phi(&mut self, group: G) -> InstrId {
    let res = Instruction::new(self, Phi(group), ~[]);
    // Prevent adding phi to block
    self.get_mut_instr(&res).added = true;
    self.phis.push(res);
    return res;
  }

  /// Perform operations on block
  pub fn with_block(&mut self,
                    id: BlockId,
                    body: &fn(b: &mut BlockBuilder<K, G, R>)) {
    let mut b = BlockBuilder {
      graph: self,
      block: id
    };
    body(&mut b);
  }

  /// Create new instruction outside the block
  pub fn new_instr(&mut self, kind: K, args: ~[InstrId]) -> InstrId {
    return Instruction::new(self, User(kind), args);
  }

  /// Set graph's root block
  pub fn set_root(&mut self, id: BlockId) {
    self.root = Some(id);
  }
}

impl<'self,
     G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R> > BlockBuilder<'self, K, G, R> {
  /// add instruction to block
  pub fn add(&mut self, kind: K, args: ~[InstrId]) -> InstrId {
    let instr_id = self.graph.new_instr(kind, args);

    self.add_existing(instr_id);

    return instr_id;
  }

  /// add existing instruction to block
  pub fn add_existing(&mut self, instr_id: InstrId) {
    assert!(!self.graph.get_instr(&instr_id).added);
    self.graph.get_mut_instr(&instr_id).added = true;
    self.graph.get_mut_instr(&instr_id).block = self.block;

    let block = self.graph.get_mut_block(&self.block);
    assert!(!block.ended);
    block.instructions.push(instr_id);
  }

  /// add arg to existing instruction in block
  pub fn add_arg(&mut self, id: InstrId, arg: InstrId) {
    assert!(self.graph.get_instr(&id).block == self.block);
    self.graph.get_mut_instr(&id).inputs.push(arg);
  }

  /// add phi movement to block
  pub fn to_phi(&mut self, input: InstrId, phi: InstrId) {
    let group = match self.graph.get_instr(&phi).kind {
      Phi(ref group) => group.clone(),
      _ => fail!("Expected Phi argument")
    };
    let out = self.graph.get_instr(&phi).output.expect("Phi output");
    let in = self.graph.get_instr(&input).output
                 .expect("Phi input output");

    // Insert one hint
    if self.graph.get_interval(&out).hint.is_none() {
      self.graph.get_mut_interval(&out).hint = Some(in);
    }

    let res = Instruction::new_empty(self.graph, ToPhi(group), ~[input]);
    self.graph.get_mut_instr(&res).output = Some(out);
    self.add_existing(res);
    self.graph.get_mut_instr(&phi).inputs.push(res);
    assert!(self.graph.get_instr(&phi).inputs.len() <= 2);
  }

  /// end block
  pub fn end(&mut self) {
    let block = self.graph.get_mut_block(&self.block);
    assert!(!block.ended);
    assert!(block.instructions.len() > 0);
    block.ended = true;
  }

  /// add `target_id` to block's successors
  pub fn goto(&mut self, target_id: BlockId) {
    self.graph.get_mut_block(&self.block).add_successor(target_id);
    self.graph.get_mut_block(&target_id).add_predecessor(self.block);
    self.end();
  }

  /// add `left` and `right` to block's successors
  pub fn branch(&mut self, left: BlockId, right: BlockId) {
    self.graph.get_mut_block(&self.block).add_successor(left)
                                         .add_successor(right);
    self.graph.get_mut_block(&left).add_predecessor(self.block);
    self.graph.get_mut_block(&right).add_predecessor(self.block);
    self.end();
  }

  /// mark block as root
  pub fn make_root(&mut self) {
    self.graph.set_root(self.block);
  }
}
