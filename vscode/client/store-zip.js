// Little Endian
var LE = true

var LocalFileHeader = 0x4034b50
var CentralDirectory = 0x2014b50

var decoder = typeof TextDecoder === 'function' ? new TextDecoder() : {
  decode: function (bytes) {
    return String.fromCharCode.apply(null, [].slice.call(new Uint8Array(bytes)))
  }
}
var encoder = typeof TextEncoder === 'function' ? new TextEncoder() : {
  encode: function (str) {
    return new Uint8Array(str.split('').map(function (c) { return c.charCodeAt() }))
  }
}

function readLocalFileHeader (view) {
  var signature = view.getUint32(0, LE)
  if (signature != LocalFileHeader) {
    throw new Error('not a local file header')
  }

  var version = view.getUint16(4, LE)
  var flag = view.getUint16(6, LE)
  var compression = view.getUint16(8, LE)
  var mtime = view.getUint16(10, LE)
  var mdate = view.getUint16(12, LE)
  var crc32 = view.getUint32(14, LE)
  var compressedSize = view.getUint32(18, LE)
  var uncompressedSize = view.getUint32(22, LE)
  var nameLength = view.getUint16(26, LE)
  var extraLength = view.getUint16(28, LE)
  var name = decoder.decode(view.buffer.slice(view.byteOffset + 30, view.byteOffset + 30 + nameLength))
  var extra = null
  if (extraLength > 0) {
    decoder.decode(view.buffer.slice(view.byteOffset + 30 + nameLength, view.byteOffset + 30 + nameLength + extraLength))
  }

  return {
    signature,
    version,
    flag,
    compression,
    mtime,
    mdate,
    crc32,
    compressedSize,
    uncompressedSize,
    name,
    extra,
    headerSize: 30 + nameLength + extraLength
  }
}

function writeLocalFileHeader (view, header) {
  const nameBytes = encoder.encode(header.name)

  view.setUint32(0, LocalFileHeader, LE)
  view.setUint16(4, header.version, LE)
  view.setUint16(6, header.flag, LE)
  view.setUint16(8, header.compression, LE)
  view.setUint16(10, header.mtime, LE)
  view.setUint16(12, header.mdate, LE)
  view.setUint32(14, header.crc32, LE)
  view.setUint32(18, header.compressedSize, LE)
  view.setUint32(22, header.uncompressedSize, LE)
  view.setUint16(26, nameBytes.byteLength, LE)
  view.setUint16(28, 0, LE)
  const writeNameTo = new Uint8Array(view.buffer, view.byteOffset + 30, 30 + nameBytes.byteLength)
  writeNameTo.set(nameBytes)

  return 30 + nameBytes.byteLength
}

function read (buffer, options) {
  if (buffer.buffer) buffer = buffer.buffer.slice(buffer.byteOffset)

  var files = []

  var offset = 0
  while (offset < buffer.byteLength) {
    var view = new DataView(buffer, offset)
    var signature = view.getUint32(0, LE)
    if (signature == LocalFileHeader) {
      var header = readLocalFileHeader(view)
      if (header.compression !== 0) {
        throw new Error('file is compressed')
      }
      offset += header.headerSize
      files.push({
        data: new Uint8Array(buffer, offset, header.compressedSize),
        header: header
      })
      offset += header.compressedSize
    } else if (signature == CentralDirectory) {
      break
    }
  }

  return files
}

function write (files) {
  var list = files
  if (!Array.isArray(files)) {
    list = Object.keys(files).map(function (name) {
      var data = files[name]
      return {
        header: {
          signature: LocalFileHeader,
          version: 10,
          flag: 0,
          compression: 0,
          mtime: 0,
          mdate: 0,
          crc32: 0,
          compressedSize: data.byteLength,
          uncompressedSize: data.byteLength,
          name,
          extra: null
        },
        data: data,
        size: 0
      }
    })
  }

  list.forEach(function (file) {
    const nameBytes = encoder.encode(file.header.name)
    file.size = 30 + nameBytes.byteLength + file.data.byteLength
  })

  var length = list.reduce(function (acc, entry) { return acc + entry.size }, 0)
  var buffer = new ArrayBuffer(length)
  var bytes = new Uint8Array(buffer)
  var offset = 0
  list.forEach(function (entry) {
    const view = new DataView(buffer, offset)
    offset += writeLocalFileHeader(view, entry.header)
    bytes.set(entry.data, offset)
    offset += entry.data.byteLength
  })

  return bytes
}

exports.read = read
exports.write = write
