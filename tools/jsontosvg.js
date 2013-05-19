function Converter(options) {
  this.input = options.input || process.stdin;
  this.output = options.output || process.stdout;

  this.offset = {
    top: 8,
    left: 8
  };
  this.annotation = {
    width: 280,
    height: 0,
    item: {
      width: 24,
      height: 24,
      padding: 8
    }
  };
  this.instruction = {
    marker: {
      width: 20,
      padding: 4
    },
    height: 20
  };
  this.block = {
    r: 3,
    titleHeight: 24
  };
  this.interval = {
    paddingX: 2,
    paddingY: 2,
    width: 16,
    height: 16,
  };
  this.use = {
    width: 5
  };
  this.arrow = {
    width: 5
  };

  this.readWhole(this.start.bind(this));
};
Converter.create = function create(options) {
  return new Converter(options);
};

Converter.prototype.readWhole = function readWhole(cb) {
  var chunks = [];
  this.input.on('readable', function() {
    var chunk;
    while (chunk = this.read()) {
      chunks.push(chunk.toString());
    }
  });
  this.input.on('end', function() {
    cb(chunks.join(''));
  });
};

Converter.prototype.tag = function tag(tag, attrs, body) {
  if (typeof attrs !== 'object') {
    body = attrs;
    attrs = {};
  }

  this.output.write('<' + tag + ' ' + Object.keys(attrs).map(function(key) {
    return key + '=' + JSON.stringify(attrs[key] + '')
  }).join(' ') + '>\n');

  if (body !== undefined) {
    if (typeof body === 'function')
      body.call(this);
    else
      this.output.write(body + '\n');
  }

  this.output.write('</' + tag + '>\n');
};

Converter.prototype.start = function start(data) {
  this.input = JSON.parse(data);

  this.tag('svg', {
    version: '1.1',
    baseProfile: 'full',
    xmlns: 'http://www.w3.org/2000/svg',
    onmouseout: 'clear()'
  }, this.draw.bind(this));
};

Converter.prototype.getX = function getX(x) {
  return this.offset.left + this.annotation.width + x;
};

Converter.prototype.getY = function getY(y) {
  return this.offset.top + y;
};

Converter.prototype.draw = function draw() {
  var self = this;

  this.drawStyles();
  this.drawScripts();

  this.drawAnnotation();
  this.drawInstructions();

  this.input.blocks.forEach(function(block) {
    this.drawBlock(block);
  }, this);

  this.input.intervals.forEach(function(interval, i) {
    this.drawInterval(interval, i);
  }, this);

  this.input.blocks.forEach(function(block) {
    this.drawArrows(block);
  }, this);
};

Converter.prototype.drawStyles = function drawStyles() {
  this.tag('defs', function() {
    this.tag('marker', {
      id: 'arrow',
      refX: 0,
      refY: 2,
      'class': 'arrow-mark',
      markerUnits: 'strokeWidth',
      markerWidth: 6,
      markerHeight: 6,
      orient: 'auto'
    }, function() {
      this.tag('path', { d: 'M 0 0 L ' + this.arrow.width + ' 2 L 0 4 Z' });
    });
  });

  this.tag('style', function() {/*<![CDATA[
    @font-face {
      font-family: "Raleway";
      font-style: normal;
      font-weight: 400;
      src: local("Raleway"),
      url(http://themes.googleusercontent.com/static/fonts/raleway/v6/cIFypx4yrWPDz3zOxk7hIQLUuEpTyoUstqEm5AMlJo4.woff) format("woff");
    }
    .annotation-wrap { fill: transparent; stroke: #333 }
    .instruction-marker { fill: transparent; }
    .instruction-marker-text, .instruction-text {
      font-family: 'Raleway';
      dominant-baseline': middle;
    }
    .instruction-marker-text {
      font-size: 8px;
    }
    .arrow { stroke: #333; fill: transparent; }
    .arrow-mark { fill: #333; }
    .block-fill { fill: #4CBFCB; }
    .interval-empty { fill: #A4EEE8; }
    .range-physical { fill: #FD6218; }
    .range-normal { fill: #FBA42B; }
    .use-any { fill: #F6E575; }
    .use-reg { fill: #315B8F; }
    .use-fixed { fill: #FD6210; }
    .highlight-interval { fill: #16DDD7; }
    .highlight-output { fill: #A40B04; }
    .highlight-input { fill: #0CF471; }
    .highlight-tmp { fill: #601D61; }
  ]]>*/}.toString().replace(/^function\s*\(\)\s*{\/\*|\*\/}$/g, ''));
};

Converter.prototype.drawScripts = function drawScripts() {
  this.tag('script', { type: 'text/ecmascript' },
           '<![CDATA[var instructions=' +
           JSON.stringify(this.input.instructions) +
           ']]>');
  this.tag('script', { type: 'text/ecmascript' }, '<![CDATA[\n' +
  function() {
    var highlighted = [];
    function each(className, cb) {
      Array.prototype.forEach.call(
        document.getElementsByClassName(className) || [],
        cb
      );
    }
    function highlight(className, color) {
      each(className, function(i) {
        i.classList.add('highlight-' + color);
        highlighted.push(function() {
          i.classList.remove('highlight-' + color);
        });
      });
    }
    function h(what, color, noclear) {
      if (!color) color = 'interval';
      if (!noclear) clear();

      // Highlight row
      if (what.r !== undefined) highlight('r-' + what.r, color);

      // Highlight column
      if (what.c !== undefined) {
        highlight('c-' + what.c, color);

        // Highlight outputs and inputs
        var instr = instructions[what.c];
        if (instr) {
          if (instr.output !== null) {
            h({ r: instr.output }, 'output', true);
          }
          instr.inputs.forEach(function(input, i) {
            h({ r: input }, 'input', true);
          });

          if (instr.temporary.length > 0) {
            instr.temporary.forEach(function(tmp, i) {
              h({ r: tmp }, 'tmp', true);
            });
          }
        }
      }
    }
    function clear() {
      for (var i = highlighted.length - 1; i >= 0; i--) {
        highlighted[i]();
      }
      highlighted = [];
    }
  }.toString().replace(/^function\s*\(\)\s*{\n?|\n?}$/g, '') + ']]>');
};

Converter.prototype.drawAnnotation = function drawAnnotation() {
  var annotation = {
    'range-physical': 'Physical register range',
    'range-normal': 'Normal range',
    'use-any': 'Any use',
    'use-reg': 'Register use',
    'use-fixed': 'Use of fixed register',
    'highlight-output': 'Instruction\'s output',
    'highlight-input': 'Instruction\'s input',
    'highlight-tmp': 'Instruction\'s temporary'
  };
  var keys = Object.keys(annotation);

  this.annotation.height = keys.length * this.annotation.item.height;

  // Draw bounding rect
  this.tag('rect', {
    x: this.offset.left - 4,
    y: this.offset.top - 2,
    width: this.annotation.width,
    height: this.annotation.height,
    'class': 'annotation-wrap'
  });

  // Just to add some margin betwen annotation and instructions
  this.annotation.height += 16;

  // Draw items
  for (var i = 0; i < keys.length; i++) {
    this.tag('rect', {
      x: this.offset.left,
      y: this.offset.top + this.annotation.item.height * i,
      width: this.annotation.item.width - this.annotation.item.padding,
      height: this.annotation.item.height - this.annotation.item.padding,
      'class': keys[i]
    });
    this.tag('text', {
      x: this.offset.left + this.annotation.item.width,
      y: this.offset.top + this.annotation.item.height * i +
         this.annotation.item.height / 2,
      'font-family': 'Raleway',
      'dominant-baseline': 'middle'
    }, annotation[keys[i]]);
  }
};

Converter.prototype.drawInstructions = function drawInstructions() {
  var self = this;
  function stringify(instr) {
    function interval(id) {
      var interval = self.input.intervals[id];
      return '<tspan class="r-' + id + '">' +
             (interval.value === 'v' ? 'v' + interval.id : interval.value) +
             '</tspan>';
    }

    var res = '';
    if (instr.output !== null)
      res += interval(instr.output) + ' = ';
    res += instr.kind + '(';
    instr.inputs.forEach(function(input, i) {
      res += interval(input);
      if (i !== instr.inputs.length - 1) res += ', ';
    });
    res += ')';
    if (instr.temporary.length > 0) {
      res += ' | tmp: ';
      instr.temporary.forEach(function(tmp, i) {
        res += interval(tmp);
        if (i !== instr.temporary.length - 1) res += ', ';
      });
    }
    return res;
  }

  Object.keys(this.input.instructions).map(function(key) {
    return parseInt(key, 10);
  }).forEach(function(key, i) {
    var markerY = this.offset.top + this.annotation.height +
                  i * this.instruction.height;
    // Draw marker
    this.tag('rect', {
      'class': 'instruction-marker c-' + key,
      x: this.offset.left,
      y: markerY,
      width: this.instruction.marker.width - this.instruction.marker.padding,
      height: this.instruction.height - this.instruction.marker.padding
    });
    this.tag('text', {
      'class': 'instruction-marker-text',
      x: this.offset.left + 4,
      y: markerY + this.instruction.height / 2
    }, key);

    // Draw text
    this.tag('text', {
      'class': 'instruction-text',
      x: this.offset.left + this.instruction.marker.width,
      y: markerY + this.instruction.height / 2
    }, stringify(this.input.instructions[key]));
  }, this);
};

Converter.prototype.getBlockRect = function getBlockRect(block) {
  var len = block.end - block.start,
      x = this.getX(block.start * this.interval.width),
      y = this.getY(0),
      width = len * this.interval.width - this.interval.paddingX,
      height = this.block.titleHeight +
               this.input.intervals.length * this.interval.height +
               this.block.r;
  return { x: x, y: y, width: width, height: height };
};

Converter.prototype.drawBlock = function drawBlock(block) {
  var rect = this.getBlockRect(block);

  // Draw block
  this.tag('rect', {
    x: rect.x,
    y: rect.y,
    rx: this.block.r,
    ry: this.block.r,
    width: rect.width,
    height: rect.height,
    'class': 'block-fill'
  });

  // Draw title
  this.tag('text', {
    'dominant-baseline': 'middle',
    'font-family': 'Raleway',
    x: rect.x + this.block.r,
    y: this.getY(this.block.titleHeight / 2)
  }, block.id);
};

Converter.prototype.drawArrows = function drawArrows(block) {
  var rect = this.getBlockRect(block);

  block.successors.forEach(function(succ) {
    // Ignore consequent blocks
    offset = cons = succ === block.id + 1 ? rect.height / 2 : 0;

    var target = this.input.blocks[succ];
    var targetRect = this.getBlockRect(target);
    this.drawArrow({
      x: rect.x + rect.width,
      y: rect.y + rect.height + this.block.r - offset,
      depth: block.loop_depth
    }, {
      x: targetRect.x,
      y: targetRect.y + targetRect.height + this.block.r - offset,
      depth: target.loop_depth
    });
  }, this);
};

Converter.prototype.drawArrow = function drawArrow(from, to) {
  var path = ['M', from.x, from.y];
  var depth = Math.log(Math.E * (1 + Math.min(from.depth, to.depth)));
  var distance = Math.log(Math.abs(to.x - from.x - this.interval.paddingX) + 1);

  // Account arrow width
  if (to.x > from.x) {
    to.x -= this.arrow.width;
    if (from.x > to.x) from.x -= this.arrow.width;
  } else {
    to.x += this.arrow.width;
    if (from.x < to.x) from.x += this.arrow.width;
  }

  var middle = {
    x: (to.x + from.x) / 2,
    y: (to.y + from.y) / 2 + 4 * depth * distance
  };

  path.push('C', middle.x, middle.y, middle.x, middle.y, to.x, to.y);
  this.tag('path', {
    d: path.join(' '),
    'class': 'arrow',
    'stroke-width': 2,
    'marker-end': 'url(#arrow)'
  });
};

Converter.prototype.drawInterval = function drawInterval(interval, i) {
  var y = this.getY(this.block.titleHeight + this.interval.height * i);

  // Draw interval
  this.input.blocks.forEach(function(block) {
    for (var c = block.start; c < block.end; c++) {
      this.tag('rect', {
        'class': 'r-' + interval.id + ' c-' + c + ' interval-empty',
        onmouseover: 'h({r:' + interval.id + ',c:' + c + '})',
        x: this.getX(c * this.interval.width),
        y: y,
        width: this.interval.width -
               (c === block.end - 1 ? this.interval.paddingX : 0),
        height: this.interval.height - this.interval.paddingY
      });
    }
  }, this);

  // Draw ranges
  interval.ranges.forEach(function(range) {
    for (var c = range.start; c < range.end; c++) {
      this.tag('rect', {
        onmouseover: 'h({r:' + interval.id + ',c:' + c + '})',
        x: this.getX(c * this.interval.width),
        y: y,
        width: this.interval.width -
               (c === range.end - 1 ? this.interval.paddingX : 0),
        height: this.interval.height - this.interval.paddingY,
        'class': interval.physical ? 'range-physical' : 'range-normal'
      });
    }
  }, this);

  // Draw uses
  interval.uses.forEach(function(use) {
    this.tag('rect', {
      onmouseover: 'h({r:' + interval.id + ',c: ' + use.pos + '})',
      x: this.getX(use.pos * this.interval.width),
      y: y,
      width: this.use.width,
      height: this.interval.height - this.interval.paddingY,
      'class': 'use-' + use.kind.type
    });
  }, this);
};

Converter.create({});
