# rms-check

Syntax checking for Age of Empires random map scripts in Node and the browser, using WebAssembly

[Install](#install) - [Usage](#usage) - [License: Apache-2.0](#license)

[![npm][npm-image]][npm-url]
[![travis][travis-image]][travis-url]
[![standard][standard-image]][standard-url]

[npm-image]: https://img.shields.io/npm/v/rms-check.svg?style=flat-square
[npm-url]: https://www.npmjs.com/package/rms-check
[travis-image]: https://img.shields.io/travis/com/goto-bus-stop/rms-check-js.svg?style=flat-square
[travis-url]: https://travis-ci.com/goto-bus-stop/rms-check-js
[standard-image]: https://img.shields.io/badge/code%20style-standard-brightgreen.svg?style=flat-square
[standard-url]: http://npm.im/standard

## Install

```
npm install rms-check
```

## Usage

```js
var { check } = require('rms-check')

var warnings = check(fs.readFileSync('./path/to/map.rms'))

if (warnings.length === 0) {
  console.log('yay')
}
```

## API

### `check(source: string): Array`

Check a source string for syntax errors and lint warnings, returning an array of problems.

The array elements are objects with properties:

 - `severity` - 1 means warning, 2 means error
 - `message` - human readable warning/error message
 - `start` - start position in the `source` where the warning applies
 - `end` - end position in the `source` where the warning applies
 - `suggestions` - an array of possible fixes, containing objects with properties:
   - `start` - start position in the `source` where the suggestion applies
   - `end` - end position in the `source` where the suggestion applies
   - `message` - human readable suggestion message
   - `replacement` - a replacement string or null. If present, the range between `start` and `end` could be replaced by this value to (probably) fix the problem.

`start` and `end` properties are objects with properties:

 - `index` - character offset into the source string
 - `line` - the line number that character is at
 - `column` - the column that character is at

## License

[Apache-2.0](LICENSE.md)
