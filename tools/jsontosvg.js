function Converter(options) {
  this.input = options.input || process.stdin;
  this.output = options.output || process.stdout;

  this.offset = {
    top: 32,
    left: 8
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
  return this.offset.left + x;
};

Converter.prototype.getY = function getY(y) {
  return this.offset.top + y;
};

Converter.prototype.draw = function draw() {
  var self = this;

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
    .arrow { stroke: rgba(0,0,0,0.6); fill: transparent; }
    .arrow-mark { fill: rgba(0,0,0,0.6); }
    .block-fill { fill: #4CBFCB; }
    .interval-empty { fill: #A4EEE8; }
    .interval-physical { fill: #FD6218; }
    .interval-normal { fill: #FBA42B; }
    .use-any { fill: #F6E575; }
    .use-register { fill: #BCDD70; }
    .use-fixed { fill: #FD621; }
    .highlight-interval { fill: #16DDD7; }
    .highlight-output { fill: #A40B04; }
    .highlight-input { fill: #0CF471; }
    .highlight-tmp { fill: #601D61; }
  ]]>*/}.toString().replace(/^function\s*\(\)\s*{\/\*|\*\/}$/g, ''));

  this.tag('text', {
    id: 'hint',
    x: this.offset.left,
    y: this.offset.top / 2,
    'dominant-baseline': 'central',
    'font-family': 'Raleway',
  }, ' ');

  this.tag('script', { type: 'text/ecmascript' },
           '<![CDATA[var instructions=' +
           JSON.stringify(this.input.instructions) +
           ']]>');
  this.tag('script', { type: 'text/ecmascript' },
           '<![CDATA[var intervals=' +
           JSON.stringify(this.input.intervals) +
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
      console.log(className);
      each(className, function(i) {
        i.classList.add('highlight-' + color);
        highlighted.push(function() {
          i.classList.remove('highlight-' + color);
        });
      });
    }
    function interval_to_str(i) {
      if (intervals[i].value === 'v') {
        return 'v' + i;
      } else {
        return intervals[i].value;
      }
    }
    function h(what, color, noclear) {
      if (!color) color = 'interval';
      if (!noclear) clear();

      // Highlight row
      if (what.r !== undefined) highlight('r-' + what.r, color);

      // Highlight column
      if (what.c !== undefined) {
        highlight('c-' + what.c, color);

        // Display hint and highlight outputs and inputs
        var instr = instructions[what.c];
        var hintText = what.c + ': ';
        if (instr) {
          if (instr.output !== null) {
            hintText += interval_to_str(instr.output) + '=';
            h({ r: instr.output }, 'output', true);
          }
          hintText += instr.kind + '(';
          instr.inputs.forEach(function(input, i) {
            hintText += interval_to_str(input);
            if (i !== instr.inputs.length - 1) hintText += ', ';
            h({ r: input }, 'input', true);
          });
          hintText += ')';

          if (instr.temporary.length > 0) {
            hintText += ' | tmp: ';
            instr.temporary.forEach(function(tmp, i) {
              hintText += interval_to_str(tmp);
              if (i !== instr.temporary.length - 1) hintText += ', ';
              h({ r: tmp }, 'tmp', true);
            });
          }
        } else {
          hintText += 'empty';
        }
        hint(hintText);
      }
    }
    var hintItem = document.getElementById('hint').firstChild;
    function hint(text) {
      hintItem.nodeValue = text;
    }
    function clear() {
      for (var i = highlighted.length - 1; i >= 0; i--) {
        highlighted[i]();
      }
      highlighted = [];
    }
  }.toString().replace(/^function\s*\(\)\s*{\n?|\n?}$/g, '') + ']]>');

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
        'class': interval.physical ? 'interval-physical' : 'interval-normal'
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
      'class': 'use-any'
    });
  }, this);
};

Converter.create({});
