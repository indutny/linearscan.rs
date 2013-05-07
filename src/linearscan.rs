use std::smallintmap::SmallIntMap;

type BlockId = uint;
type InstrId = uint;
type IntervalId = uint;
type RegisterId = uint;
type StackId = uint;

pub struct GraphBuilder<K> {
  block_id: BlockId,
  instr_id: InstrId,
  interval_id: IntervalId,
  intervals: ~SmallIntMap<~Interval>,
  blocks: ~SmallIntMap<~Block<K> >,
  instructions: ~SmallIntMap<~Instruction<K> >
}

pub struct BlockBuilder<'self, K> {
  graph: &'self mut GraphBuilder<K>,
  block: BlockId
}

struct Block<K> {
  id: BlockId,
  instructions: ~[InstrId],
  successors: ~[BlockId],
  predecessors: ~[BlockId],
  ended: bool
}

struct Instruction<K> {
  id: InstrId,
  kind: InstrKind<K>,
  output: IntervalId,
  inputs: ~[IntervalId]
}

// Abstraction to allow having user-specified instruction types
// as well as internal movement instructions
enum InstrKind<K> {
  User(K),
  Move
}

struct Interval {
  id: IntervalId,
  value: Value,
  ranges: ~[LiveRange],
  parent: Option<IntervalId>,
  children: ~[IntervalId]
}

enum Value {
  Virtual,
  Register(RegisterId),
  Stack(StackId)
}

struct LiveRange {
  start: InstrId,
  end: InstrId
}

pub impl<K> GraphBuilder<K> {
  fn new() -> GraphBuilder<K> {
    return GraphBuilder {
      block_id: 0,
      instr_id: 0,
      interval_id: 0,
      intervals: ~SmallIntMap::new(),
      blocks: ~SmallIntMap::new(),
      instructions: ~SmallIntMap::new()
    };
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

  fn verify(&mut self) {
  }

  priv fn get_block<'r>(&'r mut self, id: BlockId) -> &'r mut ~Block<K> {
    return self.blocks.find_mut(&id).unwrap();
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
    self.instr_id += 2;
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
    block.ended = true;
  }

  fn goto(&mut self, target_id: BlockId) {
    self.graph.get_block(self.block).add_successor(target_id);
    self.graph.get_block(target_id).add_predecessor(self.block);
    self.end();
  }
}

pub impl<K> Block<K> {
  fn new(graph: &mut GraphBuilder<K>) -> Block<K> {
    return Block {
      id: graph.block_id(),
      instructions: ~[],
      successors: ~[],
      predecessors: ~[],
      ended: false
    };
  }

  fn add_successor(&mut self, succ: BlockId) {
    assert!(self.successors.len() <= 2);
    self.successors.push(succ);
  }

  fn add_predecessor(&mut self, pred: BlockId) {
    assert!(self.predecessors.len() <= 2);
    self.predecessors.push(pred);
  }
}

pub impl<K> Instruction<K> {
  fn new(graph: &mut GraphBuilder<K>, kind: InstrKind<K>, args: ~[InstrId]) -> InstrId {
    let r = Instruction {
      id: graph.instr_id(),
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
  fn new(graph: &mut GraphBuilder<K>) -> IntervalId {
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
