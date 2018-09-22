const wasm = require('rms-check-wasm')

const Severity = {
  Warning: 1,
  Error: 2
}

function check (source) {
  if (typeof source !== 'string') source += ''
  return JSON.parse(wasm.check(source))
}

module.exports = {
  check,
  Severity
}
