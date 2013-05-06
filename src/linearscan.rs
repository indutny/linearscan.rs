type BlockId = uint;
type InstrId = uint;
type IntervalId = uint;
type RegisterId = uint;
type StackId = uint;

pub struct GraphBuilder<K> {
  block_id: BlockId,
  instr_id: InstrId,
  interval_id: IntervalId,
  blocks: ~[@mut Block<K>]
}

pub struct BlockBuilder<'self, K> {
  graph: &'self mut GraphBuilder<K>,
  block: @mut Block<K>
}

struct Block<K> {
  id: BlockId,
  instructions: ~[@Instruction<K>],
  successors: ~[@mut Block<K>],
  predecessors: ~[@mut Block<K>]
}

struct Instruction<K> {
  id: InstrId,
  kind: InstrKind<K>,
  output: @Interval,
  inputs: ~[@Interval]
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
  parent: Option<@Interval>,
  children: ~[@Interval]
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
  fn new() -> ~GraphBuilder<K> {
    return ~GraphBuilder {
      block_id: 0,
      instr_id: 0,
      interval_id: 0,
      blocks: ~[]
    };
  }

  fn block(&mut self, body: &fn(b: &mut BlockBuilder<K>)) {
    let b = Block::new(self);
    self.blocks.push(b);
    BlockBuilder::exec(self, b, body);
  }

  priv fn block_id(&mut self) -> BlockId {
    let r = self.block_id;
    self.block_id += 1;
    return r;
  }

  priv fn instr_id(&mut self) -> InstrId {
    let r = self.instr_id;
    self.instr_id += 2;
    return r;
  }

  priv fn interval_id(&mut self) -> IntervalId {
    let r = self.interval_id;
    self.interval_id += 1;
    return r;
  }
}

pub impl<'self, K> BlockBuilder<'self, K> {
  priv fn exec(graph: &mut GraphBuilder<K>,
               block: @mut Block<K>,
               body: &fn(b: &mut BlockBuilder<K>)) {
    let mut b = BlockBuilder {
      graph: graph,
      block: block
    };
    body(&mut b);
  }

  fn add(&'self self, kind: K, args: ~[@Instruction<K>]) -> @Instruction<K> {
    let r = Instruction::new(self.graph, User(kind), args);
    self.block.instructions.push(r);
    return r;
  }

  fn end(&self) {
  }
}

pub impl<K> Block<K> {
  fn new(graph: &mut GraphBuilder<K>) -> @mut Block<K> {
    return @mut Block {
      id: graph.block_id(),
      instructions: ~[],
      successors: ~[],
      predecessors: ~[]
    };
  }
}

pub impl<K> Instruction<K> {
  fn new(graph: &mut GraphBuilder<K>, kind: InstrKind<K>, args: ~[@Instruction<K>])
      -> @Instruction<K> {
    return @Instruction {
      id: graph.instr_id(),
      kind: kind,
      output: Interval::new(graph),
      inputs: do vec::map(args) |arg| {
        arg.output
      }
    };
  }
}

pub impl<K> Interval {
  fn new(graph: &mut GraphBuilder<K>) -> @Interval {
    return @Interval {
      id: graph.interval_id(),
      value: Virtual,
      ranges: ~[],
      parent: None,
      children: ~[]
    };
  }
}
