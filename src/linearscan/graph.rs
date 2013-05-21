use std::smallintmap::SmallIntMap;
use std::bitv::BitvSet;

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
  instructions: ~SmallIntMap<~Instruction<K> >,
  physical: ~[IntervalId]
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
  live_gen: ~BitvSet,
  live_kill: ~BitvSet,
  live_in: ~BitvSet,
  live_out: ~BitvSet,

  ended: bool
}

pub struct Instruction<K> {
  id: InstrId,
  block: BlockId,
  kind: InstrKind<K>,
  output: Option<IntervalId>,
  inputs: ~[IntervalId],
  temporary: ~[IntervalId],
  added: bool
}

// Abstraction to allow having user-specified instruction types
// as well as internal movement instructions
#[deriving(ToStr)]
pub enum InstrKind<K> {
  User(K),
  Move,
  Phi,
  ToPhi
}

pub struct Interval {
  id: IntervalId,
  value: Value,
  ranges: ~[LiveRange],
  parent: Option<IntervalId>,
  uses: ~[Use],
  children: ~[IntervalId],
  fixed: bool
}

pub enum Value {
  Virtual,
  Register(RegisterId),
  Stack(StackId)
}

pub struct Use {
  kind: UseKind,
  pos: InstrId
}

#[deriving(Eq)]
pub enum UseKind {
  UseAny,
  UseRegister,
  UseFixed(RegisterId)
}

pub struct LiveRange {
  start: InstrId,
  end: InstrId
}

pub trait KindHelper {
  fn is_call(&self) -> bool;
  fn tmp_count(&self) -> uint;
  fn use_kind(&self, i: uint) -> UseKind;
  fn result_kind(&self) -> Option<UseKind>;
}

pub impl<K: KindHelper+Copy+ToStr> Graph<K> {
  fn new() -> Graph<K> {
    Graph {
      root: 0,
      block_id: 0,
      instr_id: 0,
      interval_id: 0,
      intervals: ~SmallIntMap::new(),
      blocks: ~SmallIntMap::new(),
      instructions: ~SmallIntMap::new(),
      physical: ~[]
    }
  }

  fn empty_block(&mut self) -> BlockId {
    let block = ~Block::new(self);
    let id = block.id;
    self.blocks.insert(id, block);
    return id;
  }

  fn block(&mut self, body: &fn(b: &mut BlockBuilder<K>)) -> BlockId {
    let block = ~Block::new(self);
    let id = block.id;
    self.blocks.insert(id, block);

    // Execute body
    self.with_block(id, body);

    return id;
  }

  fn phi(&mut self) -> InstrId {
    let res = Instruction::new(self, Phi, ~[]);
    // Prevent adding phi to block
    self.get_instr(&res).added = true;
    return res;
  }

  fn with_block(&mut self, id: BlockId, body: &fn(b: &mut BlockBuilder<K>)) {
    let mut b = BlockBuilder {
      graph: self,
      block: id
    };
    body(&mut b);
  }

  fn new_instr(&mut self, kind: K, args: ~[InstrId]) -> InstrId {
    return Instruction::new(self, User(kind), args);
  }

  fn set_root(&mut self, id: BlockId) {
    self.root = id;
  }

  fn get_block<'r>(&'r mut self, id: &BlockId) -> &'r mut ~Block<K> {
    self.blocks.find_mut(id).unwrap()
  }

  fn get_instr<'r>(&'r mut self, id: &InstrId) -> &'r mut ~Instruction<K> {
    self.instructions.find_mut(id).unwrap()
  }

  fn get_interval<'r>(&'r mut self, id: &IntervalId) -> &'r mut ~Interval {
    self.intervals.find_mut(id).unwrap()
  }

  fn get_next_intersection(&self,
                           a: &IntervalId,
                           b: &IntervalId) -> Option<InstrId> {
    let int_a = self.intervals.get(a);
    let int_b = self.intervals.get(b);

    for int_a.ranges.each() |a| {
      for int_b.ranges.each() |b| {
        match a.get_intersection(b) {
          Some(pos) => {
            return Some(pos)
          },
          _ => ()
        }
      }
    }

    return None;
  }

  fn split_at(&mut self, id: &IntervalId, pos: InstrId) -> IntervalId {
    let child = Interval::new(self);
    let parent = match self.get_interval(id).parent {
      Some(parent) => parent,
      None => *id
    };

    // Find appropriate child interval
    let mut split_parent = parent;
    if !self.intervals.get(&split_parent).covers(pos) {
      for self.intervals.get(&split_parent).children.each() |child| {
        if self.intervals.get(child).covers(pos) {
          split_parent = *child;
        }
      }
      assert!(self.intervals.get(&split_parent).covers(pos));
    }

    // Add child
    self.get_interval(&parent).children.push(child);
    self.get_interval(&child).parent = Some(parent);

    // Move out ranges
    let mut child_ranges =  ~[];
    let mut range_split = false;
    let parent_ranges =
        do self.intervals.get(&split_parent).ranges.filter_mapped |range| {
      if range.end <= pos {
        Some(*range)
      } else if range.start < pos {
        // Split required
        range_split = true;
        child_ranges.push(LiveRange {
          start: pos,
          end: range.end
        });
        Some(LiveRange {
          start: range.start,
          end: pos
        })
      } else {
        child_ranges.push(*range);
        None
      }
    };

    // Ensure that at least one range is always present
    if child_ranges.len() == 0 {
      child_ranges.push(LiveRange { start: pos, end: pos });
    }
    self.get_interval(&child).ranges = child_ranges;
    self.get_interval(&split_parent).ranges = parent_ranges;

    // Move out uses
    let mut child_uses =  ~[];
    let parent_uses =
        do self.intervals.get(&split_parent).uses.filter_mapped |u| {
      if range_split && u.pos <= pos || !range_split && u.pos < pos {
        Some(*u)
      } else {
        child_uses.push(*u);
        None
      }
    };
    self.get_interval(&child).uses = child_uses;
    self.get_interval(&split_parent).uses = parent_uses;

    return child;
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

pub impl<'self, K: KindHelper+Copy+ToStr> BlockBuilder<'self, K> {
  fn add(&mut self, kind: K, args: ~[InstrId]) -> InstrId {
    let instr_id = self.graph.new_instr(kind, args);

    self.add_existing(instr_id);

    return instr_id;
  }

  fn add_existing(&mut self, instr_id: InstrId) {
    assert!(!self.graph.get_instr(&instr_id).added);
    self.graph.get_instr(&instr_id).added = true;
    self.graph.get_instr(&instr_id).block = self.block;

    let block = self.graph.get_block(&self.block);
    assert!(!block.ended);
    block.instructions.push(instr_id);
  }

  fn add_arg(&mut self, id: InstrId, arg: InstrId) {
    self.graph.get_instr(&id).inputs.push(arg);
  }

  fn to_phi(&mut self, input: InstrId, phi: InstrId) {
    match self.graph.get_instr(&phi).kind {
      Phi => (),
      _ => fail!("Expected Phi argument")
    };
    let out = self.graph.get_instr(&phi).output.expect("Phi output");

    let res = Instruction::new_empty(self.graph, ToPhi, ~[input]);
    self.graph.get_instr(&res).output = Some(out);
    self.add_existing(res);
  }

  fn end(&mut self) {
    let block = self.graph.get_block(&self.block);
    assert!(!block.ended);
    assert!(block.instructions.len() > 0);
    block.ended = true;
  }

  fn goto(&mut self, target_id: BlockId) {
    self.graph.get_block(&self.block).add_successor(target_id);
    self.graph.get_block(&target_id).add_predecessor(self.block);
    self.end();
  }

  fn branch(&mut self, left: BlockId, right: BlockId) {
    self.graph.get_block(&self.block).add_successor(left)
                                     .add_successor(right);
    self.graph.get_block(&left).add_predecessor(self.block);
    self.graph.get_block(&right).add_predecessor(self.block);
    self.end();
  }

  fn make_root(&mut self) {
    self.graph.set_root(self.block);
  }
}

pub impl<K: KindHelper+Copy+ToStr> Block<K> {
  fn new(graph: &mut Graph<K>) -> Block<K> {
    Block {
      id: graph.block_id(),
      instructions: ~[],
      successors: ~[],
      predecessors: ~[],
      loop_index: 0,
      loop_depth: 0,
      live_gen: ~BitvSet::new(),
      live_kill: ~BitvSet::new(),
      live_in: ~BitvSet::new(),
      live_out: ~BitvSet::new(),
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

pub impl<K: KindHelper+Copy+ToStr> Instruction<K> {
  fn new_empty(graph: &mut Graph<K>,
               kind: InstrKind<K>,
               args: ~[InstrId]) -> InstrId {
    let id = graph.instr_id();

    let inputs = do vec::map(args) |input_id| {
      let output = graph.instructions.get(input_id).output
                        .expect("Instruction should have output");
      graph.get_interval(&output);
      output
    };

    let mut temporary = ~[];
    for uint::range(0, kind.tmp_count()) |_| {
      temporary.push(Interval::new(graph));
    }

    let r = Instruction {
      id: id,
      block: 0,
      kind: kind,
      output: None,
      inputs: inputs,
      temporary: temporary,
      added: false
    };
    graph.instructions.insert(r.id, ~r);
    return id;
  }

  fn new(graph: &mut Graph<K>,
         kind: InstrKind<K>,
         args: ~[InstrId]) -> InstrId {

    let output = match kind.result_kind() {
      Some(_) => Some(Interval::new(graph)),
      None => None
    };

    let instr = Instruction::new_empty(graph, kind, args);
    graph.get_instr(&instr).output = output;
    return instr;
  }
}

pub impl Interval {
  fn new<K: KindHelper+Copy+ToStr>(graph: &mut Graph<K>) -> IntervalId {
    let r = Interval {
      id: graph.interval_id(),
      value: Virtual,
      ranges: ~[],
      parent: None,
      uses: ~[],
      children: ~[],
      fixed: false
    };
    let id = r.id;
    graph.intervals.insert(r.id, ~r);
    return id;
  }

  fn add_range(&mut self, start: InstrId, end: InstrId) {
    assert!(self.ranges.len() == 0 || self.ranges.head().start >= end);

    // Extend last range
    if self.ranges.len() > 0 && self.ranges.head().start == end {
      self.ranges[0].start = start;
    } else {
      // Insert new range
      self.ranges.unshift(LiveRange { start: start, end: end });
    }
  }

  fn first_range<'r>(&'r mut self) -> &'r mut LiveRange {
    assert!(self.ranges.len() != 0);
    return &mut self.ranges[0];
  }

  fn start(&self) -> InstrId {
    assert!(self.ranges.len() != 0);
    return self.ranges.head().start;
  }

  fn end(&self) -> InstrId {
    assert!(self.ranges.len() != 0);
    return self.ranges.last().end;
  }

  fn covers(&self, pos: InstrId) -> bool {
    return do self.uses.any() |u| {
      u.pos == pos
    } || do self.ranges.any() |range| {
      range.covers(pos)
    };
  }

  fn add_use(&mut self, kind: UseKind, pos: InstrId) {
    self.uses.push(Use { kind: kind, pos: pos });
  }

  fn next_fixed_use(&self, after: InstrId) -> Option<Use> {
    for self.uses.each() |u| {
      match u.kind {
        UseFixed(_) if u.pos >= after => { return Some(*u); },
        _ => ()
      }
    };
    return None;
  }

  fn next_use(&self, after: InstrId) -> Option<Use> {
    for self.uses.each() |u| {
      if u.pos >= after && u.kind != UseAny {
        return Some(*u);
      }
    };
    return None;
  }
}

impl<K: KindHelper+Copy+ToStr> KindHelper for InstrKind<K> {
  fn is_call(&self) -> bool {
    match self {
      &User(ref k) => k.is_call(),
      &Move => false,
      &ToPhi => false,
      &Phi => false
    }
  }

  fn tmp_count(&self) -> uint {
    match self {
      &User(ref k) => k.tmp_count(),
      &Move => 1,
      &Phi => 0,
      &ToPhi => 0
    }
  }

  fn use_kind(&self, i: uint) -> UseKind {
    match self {
      &User(ref k) => k.use_kind(i),
      &Move => UseAny,
      &Phi => UseAny,
      &ToPhi => UseAny
    }
  }

  fn result_kind(&self) -> Option<UseKind> {
    match self {
      &User(ref k) => k.result_kind(),
      &Move => None,
      &Phi => Some(UseAny),
      &ToPhi => Some(UseAny)
    }
  }
}

impl LiveRange {
  fn covers(&self, pos: InstrId) -> bool {
    return self.start <= pos && pos < self.end;
  }

  fn get_intersection(&self, other: &LiveRange) -> Option<InstrId> {
    if self.covers(other.start) {
      return Some(other.start);
    } else if self.covers(other.end) ||
              other.start < self.start && self.end <= other.end {
      return Some(self.start);
    }
    return None;
  }
}

impl Eq for LiveRange {
  #[inline(always)]
  fn eq(&self, other: &LiveRange) -> bool {
    self.start == other.start && self.end == other.end
  }

  #[inline(always)]
  fn ne(&self, other: &LiveRange) -> bool { !self.eq(other) }
}

// LiveRange is ordered by start position
impl Ord for LiveRange {
  #[inline(always)]
  fn lt(&self, other: &LiveRange) -> bool { self.start < other.start }

  #[inline(always)]
  fn gt(&self, other: &LiveRange) -> bool { self.start > other.start }

  #[inline(always)]
  fn le(&self, other: &LiveRange) -> bool { !self.gt(other) }

  #[inline(always)]
  fn ge(&self, other: &LiveRange) -> bool { !self.lt(other) }
}
