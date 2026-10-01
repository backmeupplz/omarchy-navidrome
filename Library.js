// Pure helpers, kept out of Service.qml so test.js can run them under node.

// Picks an album with odds proportional to its song count, so shuffling over
// the selection is uniform per song rather than per album. r is in [0, 1).
function pickAlbum(albums, excluded, r) {
  var total = 0
  for (var i = 0; i < albums.length; i++) if (!excluded[albums[i].id]) total += albums[i].songCount || 0
  var x = r * total
  for (var j = 0; j < albums.length; j++) {
    if (excluded[albums[j].id]) continue
    x -= albums[j].songCount || 0
    if (x < 0) return albums[j]
  }
  return null
}

function matches(album, query) {
  return !query || (album.name + " " + album.artist).toLowerCase().indexOf(query.toLowerCase()) !== -1
}

if (typeof module !== "undefined") module.exports = { pickAlbum: pickAlbum, matches: matches }
