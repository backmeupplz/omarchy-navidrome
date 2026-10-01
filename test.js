// node test.js
const assert = require("assert")
const { pickAlbum, matches } = require("./Library.js")

const albums = [{ id: "a", songCount: 1 }, { id: "b", songCount: 3 }, { id: "c", songCount: 0 }]
assert.equal(pickAlbum(albums, {}, 0).id, "a")
assert.equal(pickAlbum(albums, {}, 0.3).id, "b")
assert.equal(pickAlbum(albums, {}, 0.999).id, "b")
assert.equal(pickAlbum(albums, { b: true }, 0.999).id, "a")
assert.equal(pickAlbum(albums, { a: true, b: true }, 0.5), null)
assert.equal(pickAlbum([], {}, 0.5), null)

const counts = { a: 0, b: 0 }
for (let i = 0; i < 4000; i++) counts[pickAlbum(albums, {}, Math.random()).id]++
assert(counts.b > counts.a * 2, "picks follow song count")

assert(matches({ name: "Kind of Blue", artist: "Miles Davis" }, "miles"))
assert(!matches({ name: "Kind of Blue", artist: "Miles Davis" }, "coltrane"))
console.log("ok")
