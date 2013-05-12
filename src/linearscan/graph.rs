use std::smallintmap::SmallIntMap;

pub type BlockId = uint;
pub type InstrId = uint;
pub type IntervalId = uint;
pub type RegisterId = uint;
pub type StackId = uint;

pub struct Graph<K> {
  root: BlockId,
  block_id: BlockId,
  instr_id: InstrId,
  interval_id: IntervalId,
  intervals: ~SmallIntMap<~Interval>,
  blocks: ~SmallIntMap<~Block<K> >,
  instructions: ~SmallIntMap<~Instruction<K> >
}

pub struct BlockBuilder<'self, K> {
  graph: &'self mut Graph<K>,
  block: BlockId
}

pub struct Block<K> {
  id: BlockId,
  instructions: ~[InstrId],
  successors: ~[BlockId],
  predecessors: ~[BlockId],

  // Fields for flattener
  loop_index: uint,
  loop_depth: uint,

  // Fields for liveness analysis
  live_gen: ~SmallIntMap<IntervalId>,
  live_kill: ~SmallIntMap<IntervalId>,
  live_in: ~SmallIntMap<IntervalId>,
  live_out: ~SmallIntMap<IntervalId>,

  ended: bool
}

pub struct Instruction<K> {
  id: InstrId,
  flat_id: InstrId,
  kind: InstrKind<K>,
  output: IntervalId,
  inputs: ~[IntervalId]
}

// Abstraction to allow having user-specified instruction types
// as well as internal movement instructions
pub enum InstrKind<K> {
  User(K),
  Move
}

pub struct Interval {
  id: IntervalId,
  value: Value,
  ranges: ~[LiveRange],
  parent: Option<IntervalId>,
  children: ~[IntervalId]
}

pub enum Value {
  Virtual,
  Register(RegisterId),
  Stack(StackId)
}

pub struct LiveRange {
  start: InstrId,
  end: InstrId
}

pub impl<K> Graph<K> {
  fn new() -> Graph<K> {
    Graph {
      root: 0,
      block_id: 0,
      instr_id: 0,
      interval_id: 0,
      intervals: ~SmallIntMap::new(),
      blocks: ~SmallIntMap::new(),
      instructions: ~SmallIntMap::new()
    }
  }

  fn block(&mut self, body: &fn(b: &mut BlockBuilder<K>)) -> BlockId {
    let block = ~Block::new(self);
    let id = block.id;
    self.blocks.insert(id, block);

    // Execute body
    self.with_block(id, body);

    return id;
  }

  fn with_block(&mut self, id: BlockId, body: &fn(b: &mut BlockBuilder<K>)) {
    let mut b = BlockBuilder {
      graph: self,
      block: id
    };
    body(&mut b);
  }

  fn set_root(&mut self, id: BlockId) {
    self.root = id;
  }

  fn get_block<'r>(&'r mut self, id: BlockId) -> &'r mut ~Block<K> {
    self.blocks.find_mut(&id).unwrap()
  }

  fn get_instr<'r>(&'r mut self, id: InstrId) -> &'r mut ~Instruction<K> {
    self.instructions.find_mut(&id).unwrap()
  }

  #[inline(always)]
  priv fn block_id(&mut self) -> BlockId {
    let r = self.block_id;
    self.block_id += 1;
    return r;
  }

  #[inline(always)]
  priv fn instr_id(&mut self) -> InstrId {
    let r = self.instr_id;
    self.instr_id += 1;
    return r;
  }

  #[inline(always)]
  priv fn interval_id(&mut self) -> IntervalId {
    let r = self.interval_id;
    self.interval_id += 1;
    return r;
  }
}

pub impl<'self, K> BlockBuilder<'self, K> {
  fn add(&mut self, kind: K, args: ~[InstrId]) -> InstrId {
    let instr_id = Instruction::new(self.graph, User(kind), args);

    let block = self.graph.get_block(self.block);
    assert!(!block.ended);
    block.instructions.push(instr_id);

    return instr_id;
  }

  fn end(&mut self) {
    let block = self.graph.get_block(self.block);
    assert!(!block.ended);
    assert!(block.instructions.len() > 0);
    block.ended = true;
  }

  fn goto(&mut self, target_id: BlockId) {
    self.graph.get_block(self.block).add_successor(target_id);
    self.graph.get_block(target_id).add_predecessor(self.block);
    self.end();
  }

  fn branch(&mut self, left: BlockId, right: BlockId) {
    self.graph.get_block(self.block).add_successor(left)
                                    .add_successor(right);
    self.graph.get_block(left).add_predecessor(self.block);
    self.graph.get_block(right).add_predecessor(self.block);
    self.end();
  }

  fn make_root(&mut self) {
    self.graph.set_root(self.block);
  }
}

pub impl<K> Block<K> {
  fn new(graph: &mut Graph<K>) -> Block<K> {
    Block {
      id: graph.block_id(),
      instructions: ~[],
      successors: ~[],
      predecessors: ~[],
      loop_index: 0,
      loop_depth: 0,
      live_gen: ~SmallIntMap::new(),
      live_kill: ~SmallIntMap::new(),
      live_in: ~SmallIntMap::new(),
      live_out: ~SmallIntMap::new(),
      ended: false
    }
  }

  fn add_successor<'r>(&'r mut self, succ: BlockId) -> &'r mut Block<K> {
    assert!(self.successors.len() <= 2);
    self.successors.push(succ);
    return self;
  }

  fn add_predecessor(&mut self, pred: BlockId) {
    assert!(self.predecessors.len() <= 2);
    self.predecessors.push(pred);
  }
}

pub impl<K> Instruction<K> {
  fn new(graph: &mut Graph<K>, kind: InstrKind<K>, args: ~[InstrId]) -> InstrId {
    let r = Instruction {
      id: graph.instr_id(),
      flat_id: 0,
      kind: kind,
      output: Interval::new(graph),
      inputs: do vec::map(args) |id| {
        graph.instructions.get(id).output
      }
    };
    let id = r.id;
    graph.instructions.insert(r.id, ~r);
    return id;
  }
}

pub impl<K> Interval {
  fn new(graph: &mut Graph<K>) -> IntervalId {
    let r = Interval {
      id: graph.interval_id(),
      value: Virtual,
      ranges: ~[],
      parent: None,
      children: ~[]
    };
    let id = r.id;
    graph.intervals.insert(r.id, ~r);
    return id;
  }
}
