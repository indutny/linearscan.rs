use std::json::{ToJson, Json, Object, List, String, Number, Boolean, Null};
use core::hashmap::HashMap;
use linearscan::graph::{Graph, Block, Instruction, Interval, LiveRange,
                        User, Gap, ToPhi, Phi,
                        Use, UseAny, UseRegister, UseFixed,
                        Value, Virtual, Register, Stack, KindHelper};

trait JsonHelper {
  fn get_blocks(&self) -> Json;
  fn get_intervals(&self) -> Json;
  fn get_instructions(&self) -> Json;
}

impl<K: KindHelper+Copy+ToStr> ToJson for Block<K> {
  fn to_json(&self) -> Json {
    let mut obj = ~HashMap::new();

    obj.insert(~"id", Number(self.id as float));
    obj.insert(~"successors", List(do self.successors.map() |succ| {
      Number(*succ as float)
    }));

    let start = *self.instructions.head();
    let end = *self.instructions.last() + 2;
    obj.insert(~"start", Number(start as float));
    obj.insert(~"end", Number(end as float));
    obj.insert(~"loop_depth", Number(self.loop_depth as float));

    return Object(obj);
  }
}

impl<K: KindHelper+Copy+ToStr> ToJson for Instruction<K> {
  fn to_json(&self) -> Json {
    let mut obj = ~HashMap::new();

    obj.insert(~"id", Number(self.id as float));
    obj.insert(~"block", Number(self.block as float));
    obj.insert(~"kind", String(match self.kind {
      User(kind) => kind.to_str(),
      Gap => ~"~gap",
      ToPhi => ~"~to_phi",
      Phi => ~"~phi"
    }));
    obj.insert(~"inputs", List(do self.inputs.map() |input| {
      Number((*input) as float)
    }));
    obj.insert(~"temporary", List(do self.temporary.map() |t| {
      Number((*t) as float)
    }));
    obj.insert(~"output", match self.output {
      Some(output) => Number(output as float),
      None => Null
    });

    return Object(obj);
  }
}

impl ToJson for Interval {
  fn to_json(&self) -> Json {
    let mut obj = ~HashMap::new();

    obj.insert(~"id", Number(self.id as float));
    obj.insert(~"parent", match self.parent {
      Some(id) => Number(id as float),
      None => Null
    });
    obj.insert(~"children", List(do self.children.map() |child| {
      Number(*child as float)
    }));
    obj.insert(~"ranges", self.ranges.to_json());
    obj.insert(~"uses", self.uses.to_json());
    obj.insert(~"value", self.value.to_json());

    return Object(obj);
  }
}

impl ToJson for LiveRange {
  fn to_json(&self) -> Json {
    let mut obj = ~HashMap::new();

    obj.insert(~"start", Number(self.start as float));
    obj.insert(~"end", Number(self.end as float));

    return Object(obj);
  }
}

impl ToJson for Use {
  fn to_json(&self) -> Json {
    let mut obj = ~HashMap::new();
    let mut kind = ~HashMap::new();

    match self.kind {
      UseAny => kind.insert(~"type", String(~"any")),
      UseRegister => kind.insert(~"type", String(~"reg")),
      UseFixed(val) => {
        kind.insert(~"type", String(~"fixed"));
        kind.insert(~"value", val.to_json())
      }
    };
    obj.insert(~"kind", Object(kind));
    obj.insert(~"pos", Number(self.pos as float));

    return Object(obj);
  }
}

impl ToJson for Value {
  fn to_json(&self) -> Json {
    return String(match self {
      &Virtual => ~"v",
      &Register(id) => ~"r" + id.to_str(),
      &Stack(id) => ~"s" + id.to_str()
    });
  }
}

impl<K: KindHelper+Copy+ToStr> JsonHelper for Graph<K> {
  fn get_blocks(&self) -> Json {
    let mut result = ~[];

    for self.blocks.each() |_, block| {
      result.push(block.to_json());
    }

    return List(result);
  }

  fn get_intervals(&self) -> Json {
    let mut result = ~[];

    for self.intervals.each() |_, interval| {
      let mut obj = match interval.to_json() {
        Object(obj) => obj,
        _ => fail!("Unexpected interval JSON type")
      };

      obj.insert(~"physical", Boolean(interval.fixed));
      result.push(Object(obj));
    }

    return List(result);
  }

  fn get_instructions(&self) -> Json {
    let mut result = ~HashMap::new();

    for self.instructions.each() |id, instruction| {
      result.insert(id.to_str(), instruction.to_json());
    }

    return Object(result);
  }
}

impl<K: KindHelper+Copy+ToStr> ToJson for Graph<K> {
  fn to_json(&self) -> Json {
    let mut result = ~HashMap::new();

    // Export blocks
    result.insert(~"blocks", self.get_blocks());

    // Export intervals
    result.insert(~"intervals", self.get_intervals());

    // Export instructions
    result.insert(~"instructions", self.get_instructions());

    return result.to_json();
  }
}
