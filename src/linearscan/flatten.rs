use std::smallintmap::SmallIntMap;
use std::bitv::BitvSet;
use linearscan::graph::{Graph, BlockId, KindHelper};

struct MapResult {
  block: BlockId,
  score: uint
}

pub trait Flatten {
  // Perform flatten itself
  fn flatten(&mut self) -> ~[BlockId];
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

impl<K: KindHelper+Copy+ToStr> FlattenHelper for Graph<K> {
  fn flatten_get_ends(&mut self) -> ~SmallIntMap<~[BlockId]> {
    let mut queue = ~[self.root];
    let mut visited = ~BitvSet::new();
    let mut ends: ~SmallIntMap<~[BlockId]> = ~SmallIntMap::new();

    // Visit each block and find loop ends
    while queue.len() > 0 {
      let cur = queue.shift();
      visited.insert(cur);
      for self.get_block(&cur).successors.each() |succ| {
        if visited.contains(succ) {
          // Loop detected
          if ends.contains_key(succ) {
            ends.find_mut(succ).unwrap().push(cur);
          } else {
            ends.insert(*succ, ~[cur]);
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
    let mut loop_index = 0;

    for ends.each() |start, ends| {
      let mut visited = ~BitvSet::new();
      let mut queue = ~[];
      let expected_depth = self.get_block(start).loop_depth;

      for ends.each() |end| { queue.push(*end); }

      while queue.len() > 0 {
        let cur = queue.shift();
        let block = self.get_block(&cur);

        // Set depth and index of not-visited-yet nodes,
        // if we're not visiting nested loop
        if block.loop_depth == expected_depth && visited.insert(cur) {
          block.loop_index = loop_index;
          block.loop_depth += 1;
        }

        // Enqueue predecessors if current is not a loop start
        if cur != *start {
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

    for list.each() |id| {
      let mut block = self.blocks.pop(id).unwrap();

      // Update root id
      if block.id == self.root {
        self.root = block_id;
      }

      block.id = block_id;
      block_id += 1;

      // Update block id in it's instructions
      for block.instructions.each() |instr_id| {
        self.get_instr(instr_id).block = block.id;
      }

      result.push(block.id);
      queue.push(block);
    }

    // Remove all other instructions
    self.blocks.clear();

    // Insert them again
    while queue.len() > 0 {
      let block = queue.pop();
      self.blocks.insert(block.id, block);
    }

    return result;
  }

  fn flatten_reindex_instructions(&mut self, list: &[BlockId]) {
    let mut instr_id = 0;
    let mut queue = ~[];
    for list.each() |block| {
      let list = copy self.get_block(block).instructions;
      let mut new_list = ~[];

      for list.each() |id| {
        // Pop each instruction from map
        let mut instr = self.instructions.pop(id).unwrap();
        // And update its id
        instr.id = instr_id;

        // Construct new block instructions list and insert instruction into
        // new map
        new_list.push(instr.id);
        queue.push(instr);
        instr_id += 2;
      }

      // Replace block's instruction list
      self.get_block(block).instructions = new_list;
    }

    // Remove all other instructions
    self.instructions.clear();

    // Replace graph's instruction map
    while queue.len() > 0 {
      let instr = queue.pop();
      self.instructions.insert(instr.id, instr);
    }
  }
}

impl<K: KindHelper+Copy+ToStr> Flatten for Graph<K> {
  fn flatten(&mut self) -> ~[BlockId] {
    let mut queue = ~[self.root];
    let mut result = ~[];
    let mut visited = ~BitvSet::new();

    // Visit each block and find loop ends
    while queue.len() > 0 {
      let cur = queue.shift();

      // Skip visited blocks
      if visited.insert(cur) {
        result.push(cur);

        // Visit successors in loop order
        // TODO(indutny): avoid copying
        let index = self.get_block(&cur).loop_index;
        let depth = self.get_block(&cur).loop_depth;
        let successors = copy self.get_block(&cur).successors;
        match successors.len() {
          0 => (),
          1 => queue.push(successors[0]),
          2 => {
            let scores = do successors.map() |succ| {
              let block = self.get_block(succ);
              let mut res = 0;

              if index == block.loop_index {
                res += 2;
              }

              if depth <= block.loop_depth {
                res += 1;
              }

              MapResult {
                block: *succ,
                score: res
              }
            };

            if scores[0].score >= scores[1].score {
              queue.push(scores[0].block);
              queue.push(scores[1].block);
            } else {
              queue.push(scores[1].block);
              queue.push(scores[0].block);
            }
          },
          c => fail!(fmt!("Unexpected successor count: %?", c))
        };
      }
    }

    // Assign flat ids to every block
    result = self.flatten_reindex_blocks(result);

    // Assign flat ids to every instruction
    self.flatten_reindex_instructions(result);

    return result;
  }
}
