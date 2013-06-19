use extra::smallintmap::SmallIntMap;
use extra::bitv::BitvSet;
use linearscan::{KindHelper, RegisterHelper, GroupHelper};
use linearscan::graph::{Graph, BlockId};

struct MapResult {
  block: BlockId,
  score: uint
}

pub trait Flatten {
  // Perform flatten itself
  fn flatten(&mut self);
}

trait FlattenHelper {
  // Flatten CFG and detect/enumerate loops
  //
  // Get map: loop_start => [ loop ends ]
  fn flatten_get_ends(&mut self) -> ~SmallIntMap<~[BlockId]>;

  // Assign loop_index/loop_depth to each block
  fn flatten_assign_indexes(&mut self);

  // Assign new ids to blocks and instructions
  fn flatten_reindex_blocks(&mut self, list: &[BlockId]) -> ~[BlockId];
  fn flatten_reindex_instructions(&mut self, list: &[BlockId]);
}

impl<G: GroupHelper<R>,
     R: RegisterHelper<G>,
     K: KindHelper<G, R> > FlattenHelper for Graph<K, G, R> {
  fn flatten_get_ends(&mut self) -> ~SmallIntMap<~[BlockId]> {
    let mut queue = ~[self.root.expect("Root block")];
    let mut visited = ~BitvSet::new();
    let mut ends: ~SmallIntMap<~[BlockId]> = ~SmallIntMap::new();

    // Visit each block and find loop ends
    while queue.len() > 0 {
      let cur = queue.shift();
      visited.insert(cur.to_uint());
      for self.get_block(&cur).successors.each() |succ| {
        if visited.contains(&succ.to_uint()) {
          // Loop detected
          if ends.contains_key(&succ.to_uint()) {
            ends.find_mut(&succ.to_uint()).unwrap().push(cur);
          } else {
            ends.insert(succ.to_uint(), ~[cur]);
          }
        } else {
          queue.push(*succ);
        }
      }
    }

    return ends;
  }

  fn flatten_assign_indexes(&mut self) {
    let ends = self.flatten_get_ends();
    let mut loop_index = 1;

    for ends.each() |&start, ends| {
      let start_id = BlockId(start);
      let mut visited = ~BitvSet::new();
      let mut queue = ~[];
      let expected_depth = self.get_block(&start_id).loop_depth;

      // Decrement number of incoming forward branches
      assert!(self.get_block(&start_id).incoming_forward_branches == 2);
      self.get_mut_block(&start_id).incoming_forward_branches -= 1;

      for ends.each() |end| { queue.push(*end); }

      while queue.len() > 0 {
        let cur = queue.shift();
        let block = self.get_mut_block(&cur);

        // Skip visited blocks
        if !visited.insert(cur.to_uint()) { loop; }

        // Set depth and index of not-visited-yet nodes,
        // if we're not visiting nested loop
        if block.loop_depth == expected_depth {
          block.loop_index = loop_index;
          block.loop_depth += 1;
        }

        // Enqueue predecessors if current is not a loop start
        if cur.to_uint() != start {
          for block.predecessors.each() |pred| {
            queue.push(*pred);
          }
        }
      }

      // Increment loop index
      loop_index += 1;
    }
  }

  fn flatten_reindex_blocks(&mut self, list: &[BlockId]) -> ~[BlockId] {
    let mut block_id = 0;
    let mut queue = ~[];
    let mut result = ~[];
    let mut mapping = ~SmallIntMap::new();

    for list.each() |id| {
      let mut block = self.blocks.pop(&id.to_uint()).expect("block");

      // Update root id
      if block.id == self.root.expect("Root block") {
        self.root = Some(BlockId(block_id));
      }

      mapping.insert(block.id.to_uint(), BlockId(block_id));
      block.id = BlockId(block_id);
      block_id += 1;

      // Update block id in it's instructions
      for block.instructions.each() |instr_id| {
        self.get_mut_instr(instr_id).block = block.id;
      }

      result.push(block.id);
      queue.push(block);
    }

    // Remove all other instructions
    self.blocks.clear();

    // Insert them again
    while queue.len() > 0 {
      let mut block = queue.pop();
      block.successors = do block.successors.map() |succ| {
        *mapping.find(&succ.to_uint()).expect("successor")
      };
      block.predecessors = do block.predecessors.map() |pred| {
        *mapping.find(&pred.to_uint()).expect("predecessor")
      };
      self.blocks.insert(block.id.to_uint(), block);
    }

    return result;
  }

  fn flatten_reindex_instructions(&mut self, list: &[BlockId]) {
    self.instr_id = 0;
    let mut queue = ~[];
    let mut map = ~SmallIntMap::new();

    // Go through blocks and map instructions
    for list.each() |block| {
      let list = self.get_block(block).instructions.clone();
      let mut new_list = ~[];
      let start_gap = self.create_gap(block);
      new_list.push(start_gap.id);
      queue.push(start_gap);

      for list.eachi() |i, id| {
        // Pop each instruction from map
        let mut instr = self.instructions.pop(&id.to_uint()).unwrap();

        // Insert mapping
        let id = self.instr_id();
        map.insert(instr.id.to_uint(), id);

        // And update its id
        instr.id = id;

        // Construct new block instructions list and insert instruction into
        // new map
        new_list.push(instr.id);
        queue.push(instr);

        // Insert gap
        if i != list.len() - 1 {
          let gap = self.create_gap(block);
          new_list.push(gap.id);
          queue.push(gap);
        }
      }
      if list.len() != 0 {
        let end_gap = self.create_gap(block);
        new_list.push(end_gap.id);
        queue.push(end_gap);
      }

      // Replace block's instruction list
      self.get_mut_block(block).instructions = new_list;
    }

    // Add phis to queue
    let mut i = 0;
    while i < self.phis.len() {
      let mut phi = self.instructions.pop(&self.phis[i].to_uint())
                                     .expect("Phi");

      // Insert mapping
      let id = self.instr_id();
      map.insert(phi.id.to_uint(), id);

      // Update id
      phi.id = id;

      // Queue phi
      queue.push(phi);
      i += 1;
    }

    // Remove all other instructions
    self.instructions.clear();

    // Replace graph's instruction map
    while queue.len() > 0 {
      let mut instr = queue.pop();

      // Update inputs
      instr.inputs = do instr.inputs.map() |i| {
        match map.find(&i.to_uint()) {
          Some(r) => *r,
          None => *i
        }
      };

      self.instructions.insert(instr.id.to_uint(), instr);
    }
  }
}

impl<G: GroupHelper<R>,
     R: RegisterHelper<G>,
     K: KindHelper<G, R> > Flatten for Graph<K, G, R> {
  fn flatten(&mut self) {
    self.flatten_assign_indexes();

    let mut queue = ~[self.root.expect("Root block")];
    let mut list = ~[];
    let mut visited = ~BitvSet::new();

    // Visit each block and its successors
    while queue.len() > 0 {
      let cur = queue.shift();

      // Skip visited blocks
      if !visited.insert(cur.to_uint()) { loop; }

      list.push(cur);

      // Visit successors if they've no unvisited incoming forward edges
      let successors = self.get_block(&cur).successors.clone();
      for successors.each() |succ_id| {
        let succ = self.get_mut_block(succ_id);
        if succ.incoming_forward_branches == 0 {
          loop;
        }

        succ.incoming_forward_branches -= 1;
        if succ.incoming_forward_branches == 0 {
          queue.unshift(*succ_id);
        }
      }
    }

    // Assign flat ids to every block
    list = self.flatten_reindex_blocks(list);

    // Assign flat ids to every instruction
    self.flatten_reindex_instructions(list);
  }
}
