use extra::smallintmap::SmallIntMap;
use extra::bitv::BitvSet;
use std::uint;
use linearscan::{KindHelper, RegisterHelper, GroupHelper};

#[deriving(Eq, Ord, Clone)]
pub struct BlockId(uint);
#[deriving(Eq, Ord, Clone)]
pub struct InstrId(uint);
#[deriving(Eq, Ord, Clone)]
pub struct IntervalId(uint);
#[deriving(Eq, Ord, Clone)]
pub struct StackId(uint);

pub struct Graph<K, G, R> {
  root: Option<BlockId>,
  block_id: uint,
  instr_id: uint,
  interval_id: uint,
  intervals: ~SmallIntMap<~Interval<G, R> >,
  blocks: ~SmallIntMap<~Block<K> >,
  instructions: ~SmallIntMap<~Instruction<K, G> >,
  phis: ~[InstrId],
  gaps: ~SmallIntMap<~GapState>,
  prepared: bool,
  physical: ~SmallIntMap<~SmallIntMap<IntervalId> >
}

// Trait for all ids
pub trait GraphId {
  fn to_uint(&self) -> uint;
}

pub struct Block<K> {
  id: BlockId,
  instructions: ~[InstrId],
  successors: ~[BlockId],
  predecessors: ~[BlockId],

  // Fields for flattener
  loop_index: uint,
  loop_depth: uint,
  incoming_forward_branches: uint,

  // Fields for liveness analysis
  live_gen: ~BitvSet,
  live_kill: ~BitvSet,
  live_in: ~BitvSet,
  live_out: ~BitvSet,

  ended: bool
}

#[deriving(Clone)]
pub struct Instruction<K, G> {
  id: InstrId,
  block: BlockId,
  kind: InstrKind<K, G>,
  output: Option<IntervalId>,
  inputs: ~[InstrId],
  temporary: ~[IntervalId],
  added: bool
}

// Abstraction to allow having user-specified instruction types
// as well as internal movement instructions
#[deriving(ToStr, Clone)]
pub enum InstrKind<K, G> {
  User(K),
  Gap,
  Phi(G),
  ToPhi(G)
}

pub struct Interval<G, R> {
  id: IntervalId,
  value: Value<G, R>,
  hint: Option<IntervalId>,
  ranges: ~[LiveRange],
  parent: Option<IntervalId>,
  uses: ~[Use<G, R>],
  children: ~[IntervalId],
  fixed: bool
}

#[deriving(Eq, Clone)]
pub enum Value<G, R> {
  VirtualVal(G),
  RegisterVal(G, R),
  StackVal(G, StackId)
}

#[deriving(Clone)]
pub struct Use<G, R> {
  kind: UseKind<G, R>,
  pos: InstrId
}

#[deriving(Eq, Clone)]
pub enum UseKind<G, R> {
  UseAny(G),
  UseRegister(G),
  UseFixed(G, R)
}

#[deriving(Eq)]
pub struct LiveRange {
  start: InstrId,
  end: InstrId
}

pub struct GapState {
  actions: ~[GapAction]
}

#[deriving(Eq)]
pub enum GapActionKind {
  Move,
  Swap
}

pub struct GapAction {
  kind: GapActionKind,
  from: IntervalId,
  to: IntervalId
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>+Clone> Graph<K, G, R> {
  /// Create new graph
  pub fn new() -> Graph<K, G, R> {
    Graph {
      root: None,
      block_id: 0,
      instr_id: 0,
      interval_id: 0,
      intervals: ~SmallIntMap::new(),
      blocks: ~SmallIntMap::new(),
      instructions: ~SmallIntMap::new(),
      phis: ~[],
      gaps: ~SmallIntMap::new(),
      prepared: false,
      physical: ~SmallIntMap::new()
    }
  }

  /// Create gap (internal)
  pub fn create_gap(&mut self, block: &BlockId) -> ~Instruction<K, G> {
    let id = self.instr_id();
    return ~Instruction {
      id: id,
      block: *block,
      kind: Gap,
      output: None,
      inputs: ~[],
      temporary: ~[],
      added: true
    };
  }

  /// Mutable block getter
  pub fn get_mut_block<'r>(&'r mut self, id: &BlockId) -> &'r mut ~Block<K> {
    self.blocks.find_mut(&id.to_uint()).unwrap()
  }

  pub fn get_block<'r>(&'r self, id: &BlockId) -> &'r ~Block<K> {
    self.blocks.get(&id.to_uint())
  }

  /// Return ordered list of blocks
  pub fn get_block_list(&self) -> ~[BlockId] {
    let mut blocks = ~[];
    for self.blocks.each() |_, block| {
      blocks.push(block.id);
    }
    return blocks;
  }

  /// Mutable instruction getter
  pub fn get_mut_instr<'r>(&'r mut self,
                           id: &InstrId) -> &'r mut ~Instruction<K, G> {
    self.instructions.find_mut(&id.to_uint()).unwrap()
  }

  pub fn get_instr<'r>(&'r self, id: &InstrId) -> &'r ~Instruction<K, G> {
    self.instructions.get(&id.to_uint())
  }

  /// Instruction output getter
  pub fn get_output(&self, id: &InstrId) -> IntervalId {
    self.instructions.get(&id.to_uint()).output.expect("Instruction output")
  }

  /// Mutable interval getter
  pub fn get_mut_interval<'r>(&'r mut self,
                              id: &IntervalId) -> &'r mut ~Interval<G, R> {
    self.intervals.find_mut(&id.to_uint()).unwrap()
  }

  pub fn get_interval<'r>(&'r self, id: &IntervalId) -> &'r ~Interval<G, R> {
    self.intervals.get(&id.to_uint())
  }

  /// Mutable gap state getter
  pub fn get_mut_gap<'r>(&'r mut self, id: &InstrId) -> &'r mut ~GapState {
    if !self.gaps.contains_key(&id.to_uint()) {
      self.gaps.insert(id.to_uint(), ~GapState { actions: ~[] });
    }
    self.gaps.find_mut(&id.to_uint()).unwrap()
  }

  /// Find next intersection of two intervals
  pub fn get_intersection(&self,
                          a: &IntervalId,
                          b: &IntervalId) -> Option<InstrId> {
    let int_a = self.get_interval(a);
    let int_b = self.get_interval(b);

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

  /// Return `true` if `pos` is either some block's start or end
  pub fn block_boundary(&self, pos: InstrId) -> bool {
    let block = self.get_block(&self.get_instr(&pos).block);
    return block.start() == pos || block.end() == pos;
  }

  /// Find optimal split position between two instructions
  pub fn optimal_split_pos(&self,
                           group: &G,
                           start: InstrId,
                           end: InstrId) -> InstrId {
    // Fast and unfortunate case
    if start == end {
      return end;
    }

    let mut best_pos = end;
    let mut best_depth = uint::max_value;
    for self.blocks.each() |_, block| {
      if best_depth >= block.loop_depth {
        let block_to = block.end();

        // Choose the most shallow block
        if start < block_to && block_to <= end {
          best_pos = block_to;
          best_depth = block.loop_depth;
        }
      }
    }

    // Always split at gap
    if !self.is_gap(&best_pos) && !self.clobbers(group, &best_pos) {
      assert!(best_pos.to_uint() >= start.next().to_uint());
      best_pos = best_pos.prev();
    }
    assert!(start < best_pos && best_pos <= end);
    return best_pos;
  }

  /// Split interval or one of it's children at specified position, return
  /// id of split child.
  pub fn split_at(&mut self, id: &IntervalId, pos: InstrId) -> IntervalId {
    // We should always make progress
    assert!(self.get_interval(id).start() < pos);

    // Split could be either at gap or at call
    let group = self.get_interval(id).value.group();
    assert!(self.is_gap(&pos) || self.clobbers(&group, &pos));

    let child = Interval::new(self, group.clone());
    let parent = match self.get_interval(id).parent {
      Some(parent) => parent,
      None => *id
    };

    // Find appropriate child interval
    let mut split_parent = parent;
    if !self.get_interval(&split_parent).covers(pos) {
      for self.get_interval(&split_parent).children.each() |child| {
        if self.get_interval(child).covers(pos) {
          split_parent = *child;
        }
      }
      assert!(self.get_interval(&split_parent).covers(pos));
    }

    // Insert movement
    let split_at_call = self.clobbers(&group, &pos);
    if split_at_call || !self.block_boundary(pos) {
      self.get_mut_gap(&pos).add_move(&split_parent, &child);
    }

    // Move out ranges
    let mut child_ranges =  ~[];
    let parent_ranges =
        do self.get_interval(&split_parent).ranges.filter_mapped |range| {
      if range.end <= pos {
        Some(*range)
      } else if range.start < pos {
        // Split required
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
    assert!(child_ranges.len() != 0);
    assert!(parent_ranges.len() != 0);
    self.get_mut_interval(&child).ranges = child_ranges;
    self.get_mut_interval(&split_parent).ranges = parent_ranges;

    // Insert register hint
    self.get_mut_interval(&child).hint = Some(split_parent);

    // Move out uses
    let mut child_uses =  ~[];
    let split_on_call = self.get_instr(&pos).kind.clobbers(&group);

    // XXX: Wait for rust bug to be fixed and use filter_mapped
    let mut parent_uses = self.get_interval(&split_parent).uses.clone();
    do parent_uses.retain |u| {
      if split_on_call && u.pos <= pos || !split_on_call && u.pos < pos {
        true
      } else {
        child_uses.push(u.clone());
        false
      }
    };
    self.get_mut_interval(&child).uses = child_uses;
    self.get_mut_interval(&split_parent).uses = parent_uses;

    // Add child
    let mut index = 0;
    for self.get_interval(&parent).children
            .rev_iter().enumerate().advance |(i, child)| {
      if self.get_interval(child).end() <= pos {
        index = i + 1;
        break;
      }
    };
    self.get_mut_interval(&parent).children.insert(index, child);
    self.get_mut_interval(&child).parent = Some(parent);

    return child;
  }

  /// Helper function
  pub fn iterate_children(&self,
                          id: &IntervalId,
                          f: &fn(&~Interval<G, R>) -> bool) -> bool {
    let p = self.get_interval(id);
    if !f(p) {
      return false;
    }

    for p.children.each() |child_id| {
      let child = self.get_interval(child_id);
      if !f(child) { break; }
    }

    true
  }

  /// Find child interval, that covers specified position
  pub fn child_at(&self,
                  parent: &IntervalId,
                  pos: InstrId) -> Option<IntervalId> {
    for self.iterate_children(parent) |interval| {
      if interval.start() <= pos && pos < interval.end() {
        return Some(interval.id);
      }
    };

    // No match?
    None
  }

  pub fn child_with_use_at(&self,
                           parent: &IntervalId,
                           pos: InstrId) -> Option<IntervalId> {
    for self.iterate_children(parent) |interval| {
      if interval.start() <= pos && pos <= interval.end() &&
         interval.uses.any(|u| { u.pos == pos }) {
        return Some(interval.id);
      }
    };

    // No match?
    None
  }

  pub fn get_value(&self,
                   i: &IntervalId,
                   pos: InstrId) -> Option<Value<G, R> > {
    let child = self.child_with_use_at(i, pos);
    match child {
      Some(child) => Some(self.get_interval(&child).value.clone()),
      None => None
    }
  }

  /// Return true if instruction at specified position is Gap
  pub fn is_gap(&self, pos: &InstrId) -> bool {
    match self.get_instr(pos).kind {
      Gap => true,
      _ => false
    }
  }

  /// Return true if instruction at specified position contains
  /// register-clobbering call.
  pub fn clobbers(&self, group: &G, pos: &InstrId) -> bool {
    return self.get_instr(pos).kind.clobbers(group);
  }

  /// Return next block id, used at graph construction
  #[inline(always)]
  priv fn block_id(&mut self) -> BlockId {
    let r = self.block_id;
    self.block_id += 1;
    return BlockId(r);
  }

  /// Return next instruction id, used at graph construction
  #[inline(always)]
  pub fn instr_id(&mut self) -> InstrId {
    let r = self.instr_id;
    self.instr_id += 1;
    return InstrId(r);
  }

  /// Return next interval id, used at graph construction
  #[inline(always)]
  priv fn interval_id(&mut self) -> IntervalId {
    let r = self.interval_id;
    self.interval_id += 1;
    return IntervalId(r);
  }
}

impl<G: GroupHelper, R: RegisterHelper<G>, K: KindHelper<G, R>+Clone> Block<K> {
  /// Create new empty block
  pub fn new(graph: &mut Graph<K, G, R>) -> Block<K> {
    Block {
      id: graph.block_id(),
      instructions: ~[],
      successors: ~[],
      predecessors: ~[],
      loop_index: 0,
      loop_depth: 0,
      incoming_forward_branches: 0,
      live_gen: ~BitvSet::new(),
      live_kill: ~BitvSet::new(),
      live_in: ~BitvSet::new(),
      live_out: ~BitvSet::new(),
      ended: false
    }
  }

  pub fn add_successor<'r>(&'r mut self, succ: BlockId) -> &'r mut Block<K> {
    assert!(self.successors.len() <= 2);
    self.successors.push(succ);
    return self;
  }

  pub fn add_predecessor(&mut self, pred: BlockId) {
    assert!(self.predecessors.len() <= 2);
    self.predecessors.push(pred);
    // NOTE: we'll decrease them later in flatten.rs
    self.incoming_forward_branches += 1;
  }
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>+Clone> Instruction<K, G> {
  /// Create instruction without output interval
  pub fn new_empty(graph: &mut Graph<K, G, R>,
                   kind: InstrKind<K, G>,
                   args: ~[InstrId]) -> InstrId {
    let id = graph.instr_id();

    let mut temporary = ~[];
    for kind.temporary().each() |group| {
      temporary.push(Interval::new(graph, group.clone()));
    }

    let r = Instruction {
      id: id,
      block: BlockId(0), // NOTE: this will be overwritten soon
      kind: kind,
      output: None,
      inputs: copy args,
      temporary: temporary,
      added: false
    };
    graph.instructions.insert(r.id.to_uint(), ~r);
    return id;
  }

  /// Create instruction with output
  pub fn new(graph: &mut Graph<K, G, R>,
             kind: InstrKind<K, G>,
             args: ~[InstrId]) -> InstrId {

    let output = match kind.result_kind() {
      Some(k) => Some(Interval::new(graph, k.group())),
      None => None
    };

    let instr = Instruction::new_empty(graph, kind, args);
    graph.get_mut_instr(&instr).output = output;
    return instr;
  }
}

impl<G: GroupHelper, R: RegisterHelper<G> > Interval<G, R> {
  /// Create new virtual interval
  pub fn new<K: KindHelper<G, R>+Clone>(graph: &mut Graph<K, G, R>,
                                        group: G) -> IntervalId {
    let r = Interval {
      id: graph.interval_id(),
      value: VirtualVal(group),
      hint: None,
      ranges: ~[],
      parent: None,
      uses: ~[],
      children: ~[],
      fixed: false
    };
    let id = r.id;
    graph.intervals.insert(r.id.to_uint(), ~r);
    return id;
  }

  /// Add range to interval's live range list.
  /// NOTE: Ranges are ordered by start position
  pub fn add_range(&mut self, start: InstrId, end: InstrId) {
    assert!(self.ranges.len() == 0 || self.ranges.head().start >= end);

    // Extend last range
    if self.ranges.len() > 0 && self.ranges.head().start == end {
      self.ranges[0].start = start;
    } else {
      // Insert new range
      self.ranges.unshift(LiveRange { start: start, end: end });
    }
  }

  /// Return mutable first range
  pub fn first_range<'r>(&'r mut self) -> &'r mut LiveRange {
    assert!(self.ranges.len() != 0);
    return &mut self.ranges[0];
  }

  /// Return interval's start position
  pub fn start(&self) -> InstrId {
    assert!(self.ranges.len() != 0);
    return self.ranges.head().start;
  }

  /// Return interval's end position
  pub fn end(&self) -> InstrId {
    assert!(self.ranges.len() != 0);
    return self.ranges.last().end;
  }

  /// Return true if one of the ranges contains `pos`
  pub fn covers(&self, pos: InstrId) -> bool {
    return do self.ranges.any() |range| {
      range.covers(pos)
    };
  }

  /// Add use to the interval's use list.
  /// NOTE: uses are ordered by increasing `pos`
  pub fn add_use(&mut self, kind: UseKind<G, R>, pos: InstrId) {
    assert!(self.uses.len() == 0 ||
            self.uses[0].pos > pos ||
            self.uses[0].kind.group() == kind.group());
    self.uses.unshift(Use { kind: kind, pos: pos });
  }

  /// Return next UseFixed(...) after `after` position.
  pub fn next_fixed_use(&self, after: InstrId) -> Option<Use<G, R> > {
    for self.uses.each() |u| {
      match u.kind {
        UseFixed(_, _) if u.pos >= after => { return Some(u.clone()); },
        _ => ()
      }
    };
    return None;
  }

  /// Return next UseFixed(...) or UseRegister after `after` position.
  pub fn next_use(&self, after: InstrId) -> Option<Use<G, R> > {
    for self.uses.each() |u| {
      if u.pos >= after && !u.kind.is_any() {
        return Some(u.clone());
      }
    };
    return None;
  }

  /// Return last UseFixed(...) or UseRegister before `before` position
  pub fn last_use(&self, before: InstrId) -> Option<Use<G, R> > {
    for self.uses.rev_iter().advance |u| {
      if u.pos <= before && !u.kind.is_any() {
        return Some(u.clone());
      }
    };
    return None;
  }
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>+Clone> KindHelper<G, R> for InstrKind<K, G> {
  /// Return true if instruction is clobbering registers
  pub fn clobbers(&self, group: &G) -> bool {
    match self {
      &User(ref k) => k.clobbers(group),
      &Gap => false,
      &ToPhi(_) => false,
      &Phi(_) => false
    }
  }

  /// Return count of instruction's temporary operands
  pub fn temporary(&self) -> ~[G] {
    match self {
      &User(ref k) => k.temporary(),
      &Gap => ~[],
      &Phi(_) => ~[],
      &ToPhi(_) => ~[]
    }
  }

  /// Return use kind of instruction's `i`th input
  pub fn use_kind(&self, i: uint) -> UseKind<G, R> {
    match self {
      &User(ref k) => k.use_kind(i),
      // note: group is not important for gap
      &Gap => UseAny(GroupHelper::any()),
      &Phi(ref g) => UseAny(g.clone()),
      &ToPhi(ref g) => UseAny(g.clone())
    }
  }

  /// Return result kind of instruction or None, if instruction has no result
  pub fn result_kind(&self) -> Option<UseKind<G, R> > {
    match self {
      &User(ref k) => k.result_kind(),
      &Gap => None,
      &Phi(ref g) => Some(UseAny(g.clone())),
      &ToPhi(ref g) => Some(UseAny(g.clone()))
    }
  }
}

impl LiveRange {
  /// Return true if range contains position
  pub fn covers(&self, pos: InstrId) -> bool {
    return self.start <= pos && pos < self.end;
  }

  /// Return first intersection position of two ranges
  pub fn get_intersection(&self, other: &LiveRange) -> Option<InstrId> {
    if self.covers(other.start) {
      return Some(other.start);
    } else if other.start < self.start && self.start < other.end {
      return Some(self.start);
    }
    return None;
  }
}

impl<G: GroupHelper, R: RegisterHelper<G> > Value<G, R> {
  pub fn is_virtual(&self) -> bool {
    match self {
      &VirtualVal(_) => true,
      _ => false
    }
  }

  pub fn group(&self) -> G {
    match self {
      &VirtualVal(ref g) => g.clone(),
      &RegisterVal(ref g, _) => g.clone(),
      &StackVal(ref g, _) => g.clone()
    }
  }
}

impl<G: GroupHelper, R: RegisterHelper<G> > UseKind<G, R> {
  pub fn is_fixed(&self) -> bool {
    match self {
      &UseFixed(_, _) => true,
      _ => false
    }
  }

  pub fn is_any(&self) -> bool {
    match self {
      &UseAny(_) => true,
      _ => false
    }
  }

  pub fn group(&self) -> G {
    match self {
      &UseRegister(ref g) => g.clone(),
      &UseAny(ref g) => g.clone(),
      &UseFixed(ref g, _) => g.clone()
    }
  }
}

impl GapState {
  pub fn add_move(&mut self, from: &IntervalId, to: &IntervalId) {
    self.actions.push(GapAction { kind: Move, from: *from, to: *to });
  }
}

impl<K> Block<K> {
  pub fn start(&self) -> InstrId {
    assert!(self.instructions.len() != 0);
    return *self.instructions.head();
  }

  pub fn end(&self) -> InstrId {
    assert!(self.instructions.len() != 0);
    return self.instructions.last().next();
  }
}

// Implement trait for ids
impl GraphId for BlockId {
  fn to_uint(&self) -> uint { match self { &BlockId(id) => id } }
}

impl GraphId for InstrId {
  fn to_uint(&self) -> uint { match self { &InstrId(id) => id } }
}

impl InstrId {
  pub fn prev(&self) -> InstrId { InstrId(self.to_uint() - 1 ) }
  pub fn next(&self) -> InstrId { InstrId(self.to_uint() + 1 ) }
}

impl GraphId for IntervalId {
  fn to_uint(&self) -> uint { match self { &IntervalId(id) => id } }
}

impl GraphId for StackId {
  fn to_uint(&self) -> uint { match self { &StackId(id) => id } }
}
