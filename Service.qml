import QtQuick
import Quickshell
import Quickshell.Io
import "Library.js" as Lib

// Talks Subsonic to the server and drives one headless mpv over its JSON IPC
// socket. The bar is built per monitor; every widget reads this singleton.
// mpv-mpris (Omarchy ships it) exposes playback to media keys and the media
// widget for free.
Item {
  id: root

  property var shell: null

  readonly property string configPath: Quickshell.env("HOME") + "/.config/omarchy/navidrome.json"
  readonly property string socketPath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/omarchy-navidrome-mpv.sock"

  // Only the salted token is stored, never the password (Subsonic token auth).
  property string server: ""
  property string user: ""
  property string salt: ""
  property string token: ""
  readonly property bool configured: server !== "" && token !== ""

  property var albums: []          // [{id, name, artist, year, songCount, coverArt}]
  property var excluded: ({})      // albumId -> true; everything else is in the shuffle pool
  property int excludedCount: 0
  property var albumSongs: ({})    // albumId -> songs, fetched on demand
  property bool loading: false
  property string error: ""

  // queue mirrors mpv's playlist index for index.
  property var queue: []
  property int index: -1
  readonly property var current: index >= 0 && index < queue.length ? queue[index] : null
  property bool paused: false
  property real position: 0
  property real duration: 0
  property bool shuffle: false
  property var recent: []
  property bool picking: false

  // Lo-fi: tracks rendered locally by lofi/lofi (Rust), one ahead, played through the same mpv queue.
  readonly property var vibes: [
    { value: "chill", label: "Chill" }, { value: "rainy", label: "Rainy" }, { value: "sunny", label: "Sunny" },
    { value: "late", label: "Late" }, { value: "jazz", label: "Jazz" }, { value: "study", label: "Study" }
  ]
  readonly property string lofiBin: decodeURIComponent(Qt.resolvedUrl("lofi/lofi").toString().replace(/^file:\/\//, ""))
  readonly property string lofiDir: (Quickshell.env("XDG_CACHE_HOME") || Quickshell.env("HOME") + "/.cache") + "/omarchy-navidrome"
  property string lofi: ""          // vibe playing; "" when playing the library
  property string lofiVibe: "chill" // last vibe picked, saved
  property bool lofiStart: false    // the next render replaces playback instead of queueing
  readonly property bool rendering: renderer.running

  // The checked albums mirrored to a server playlist, so other clients get the same pool.
  readonly property string playlistName: "Shuffle Selection"
  property var albumTracks: ({})    // albumId -> song ids in disc/track order
  property int trackCount: 0
  property string playlistId: ""
  property int playlistCount: 0
  property string synced: ""        // the song ids last written
  property bool syncing: false

  // --- Subsonic ------------------------------------------------------------

  function query(params) {
    var q = "u=" + encodeURIComponent(user) + "&t=" + token + "&s=" + salt + "&v=1.16.1&c=omarchy-navidrome&f=json"
    for (var k in params) {
      var vals = Array.isArray(params[k]) ? params[k] : [params[k]]
      for (var i = 0; i < vals.length; i++) q += "&" + k + "=" + encodeURIComponent(vals[i])
    }
    return q
  }

  function url(method, params) { return server + "/rest/" + method + "?" + query(params) }

  function coverUrl(id, size) {
    return id && configured ? url("getCoverArt", { id: id, size: size }) : ""
  }

  // post sends the params as a form body, for song lists too long for a URL.
  function api(method, params, done, post) {
    var xhr = new XMLHttpRequest()
    xhr.onreadystatechange = function() {
      if (xhr.readyState !== XMLHttpRequest.DONE) return
      var r = null
      try { r = JSON.parse(xhr.responseText)["subsonic-response"] } catch (e) {}
      if (r && r.status === "ok") {
        root.error = ""
        done(r)
      } else {
        root.error = r && r.error ? r.error.message : "Can't reach " + root.server
        done(null)
      }
    }
    if (post) {
      xhr.open("POST", server + "/rest/" + method)
      xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded")
      xhr.send(query(params))
    } else {
      xhr.open("GET", url(method, params))
      xhr.send()
    }
  }

  function login(server, user, password) {
    server = server.trim().replace(/\/+$/, "")
    if (!/^https?:\/\//.test(server)) server = "https://" + server
    root.server = server
    root.user = user.trim()
    root.salt = Math.random().toString(36).slice(2, 12)
    root.token = Qt.md5(password + root.salt)
    loading = true
    api("ping", {}, function(r) {
      loading = false
      if (!r) { root.token = ""; return } // keeps the form up on a typo
      save()
      loadAlbums()
    })
  }

  function logout() {
    stop()
    token = ""
    albums = []
    albumSongs = {}
    albumTracks = {}
    trackCount = 0
    playlistId = ""
    synced = ""
    save()
  }

  function loadAlbums() {
    if (!configured) return
    loading = true
    var acc = []
    var page = function(offset) {
      api("getAlbumList2", { type: "alphabeticalByArtist", size: 500, offset: offset }, function(r) {
        if (!r) { loading = false; return }
        var list = (r.albumList2 && r.albumList2.album) || []
        for (var i = 0; i < list.length; i++) {
          var a = list[i]
          acc.push({ id: a.id, name: a.name || "", artist: a.artist || "", year: a.year || 0, songCount: a.songCount || 0, coverArt: a.coverArt || "" })
        }
        if (list.length === 500) page(offset + 500)
        else { albums = acc; loading = false; loadTracks() }
      })
    }
    page(0)
  }

  // Every song's album, in a few bulk requests (an empty search3 lists the whole library on Navidrome).
  function loadTracks() {
    var acc = {}, n = 0
    var page = function(offset) {
      api("search3", { query: "", songCount: 500, songOffset: offset, albumCount: 0, artistCount: 0 }, function(r) {
        if (!r) return
        var list = (r.searchResult3 && r.searchResult3.song) || []
        for (var i = 0; i < list.length; i++) {
          var s = list[i]
          if (!s.albumId) continue
          ;(acc[s.albumId] = acc[s.albumId] || []).push([(s.discNumber || 1) * 10000 + (s.track || 0), s.id])
          n++
        }
        if (list.length === 500) return page(offset + 500)
        var ids = {}
        for (var a in acc) ids[a] = acc[a].sort(function(x, y) { return x[0] - y[0] }).map(function(x) { return x[1] })
        albumTracks = ids
        trackCount = n
        findPlaylist()
      })
    }
    page(0)
  }

  // --- shuffle playlist ----------------------------------------------------

  function findPlaylist() {
    api("getPlaylists", {}, function(r) {
      if (!r) return
      var list = (r.playlists && r.playlists.playlist) || []
      for (var i = 0; i < list.length; i++) {
        if (list[i].name === root.playlistName && list[i].owner === root.user) {
          root.playlistId = list[i].id
          root.playlistCount = list[i].songCount || 0
        }
      }
      syncPlaylist()
    })
  }

  function syncPlaylist() {
    // trackCount guards against servers whose search3 can't list everything: never empty the playlist on that.
    if (!configured || !trackCount) return
    if (syncing) { syncTimer.restart(); return }
    var ids = []
    for (var i = 0; i < albums.length; i++) if (!excluded[albums[i].id]) Array.prototype.push.apply(ids, albumTracks[albums[i].id] || [])
    var key = ids.join(",")
    if (key === synced) return
    var done = function(r) {
      root.syncing = false
      if (!r) return
      if (r.playlist) { root.playlistId = r.playlist.id; root.playlistCount = r.playlist.songCount || 0 }
      else root.playlistCount = 0
      root.synced = key
    }
    syncing = true
    if (!ids.length) {
      if (!playlistId) return done({})
      var all = []
      for (var k = 0; k < playlistCount; k++) all.push(k)
      return api("updatePlaylist", { playlistId: playlistId, songIndexToRemove: all }, done, true)
    }
    // With playlistId, createPlaylist replaces the songs. If the playlist was deleted elsewhere, make a new one.
    api("createPlaylist", playlistId ? { playlistId: playlistId, songId: ids } : { name: playlistName, songId: ids }, function(r) {
      if (r || !root.playlistId) return done(r)
      root.playlistId = ""
      api("createPlaylist", { name: root.playlistName, songId: ids }, done, true)
    }, true)
  }

  Timer {
    id: syncTimer
    interval: 1500
    onTriggered: root.syncPlaylist()
  }

  function songs(albumId, done) {
    if (albumSongs[albumId]) return done(albumSongs[albumId])
    api("getAlbum", { id: albumId }, function(r) {
      if (!r) return done([])
      albumSongs[albumId] = r.album.song || []
      done(albumSongs[albumId])
    })
  }

  // --- shuffle pool --------------------------------------------------------

  function isSelected(albumId) { return !excluded[albumId] }

  function setSelected(albumId, on) {
    var next = Object.assign({}, excluded)
    if (on) delete next[albumId]
    else next[albumId] = true
    setExcluded(next)
  }

  function selectAll(on) {
    var next = {}
    if (!on) for (var i = 0; i < albums.length; i++) next[albums[i].id] = true
    setExcluded(next)
  }

  function setExcluded(next) {
    excluded = next
    excludedCount = Object.keys(next).length
    saveTimer.restart()
    syncTimer.restart()
  }

  // Draws one song uniformly from the selected albums, avoiding recent repeats.
  // ponytail: one getAlbum per new album drawn; cached after, fine for any library size.
  function pickRandom(done, attempt) {
    attempt = attempt || 0
    var album = Lib.pickAlbum(albums, excluded, Math.random())
    if (!album) return done(null)
    songs(album.id, function(list) {
      if (!list.length) return attempt < 10 ? pickRandom(done, attempt + 1) : done(null)
      var song = list[Math.floor(Math.random() * list.length)]
      if (recent.indexOf(song.id) !== -1 && attempt < 10) return pickRandom(done, attempt + 1)
      done(song)
    })
  }

  function startShuffle() {
    if (picking) return
    picking = true
    pickRandom(function(song) {
      picking = false
      if (song) play([song], 0, true)
    })
  }

  function appendRandom() {
    if (picking) return
    picking = true
    pickRandom(function(song) {
      picking = false
      if (!song || !shuffle) return
      queue = queue.concat([song])
      // append-play also restarts mpv if the last track ran out before the pick landed.
      mpv(["loadfile", streamUrl(song), "append-play"])
    })
  }

  // --- playback ------------------------------------------------------------

  function streamUrl(song) { return song.path || url("stream", { id: song.id }) }

  function mpv(args) {
    if (!ipc.connected) return
    ipc.write(JSON.stringify({ command: args }) + "\n")
    ipc.flush()
  }

  function play(list, start, isShuffle) {
    if (!list.length) return
    shuffle = !!isShuffle
    lofi = ""
    lofiStart = false
    queue = list
    for (var i = 0; i < list.length; i++) mpv(["loadfile", streamUrl(list[i]), i === 0 ? "replace" : "append"])
    if (start) mpv(["set_property", "playlist-pos", start])
    mpv(["set_property", "pause", false])
  }

  function playAlbum(albumId, start) {
    songs(albumId, function(list) { play(list, start || 0, false) })
  }

  function togglePause() { if (current) mpv(["cycle", "pause"]) }
  function next() { if (current) mpv(["playlist-next", "force"]) }
  function previous() { if (current) mpv(position > 5 ? ["seek", 0, "absolute"] : ["playlist-prev", "force"]) }
  function seek(seconds) { mpv(["seek", seconds, "absolute"]) }
  function stop() { shuffle = false; lofi = ""; lofiStart = false; queue = []; mpv(["stop"]) }

  function onTrack(pos) {
    index = pos
    position = 0
    if (!current) return
    recent = recent.concat([current.id]).slice(-recentLimit())
    // ponytail: scrobbles at track start; move to the halfway mark if play counts matter.
    if (!current.path) api("scrobble", { id: current.id, submission: true }, function() {})
    if (shuffle && index === queue.length - 1) appendRandom()
    if (lofi && index === queue.length - 1) renderLofi()
  }

  // --- lo-fi ---------------------------------------------------------------

  function startLofi(vibe) {
    if (!vibes.some(function(v) { return v.value === vibe })) vibe = lofiVibe
    shuffle = false
    lofi = vibe
    lofiVibe = vibe
    lofiStart = true
    saveTimer.restart()
    renderLofi()
  }

  // Renders the next track for the current vibe: it replaces playback after startLofi, otherwise it queues.
  // A render that finishes after the vibe changed is dropped and the right one starts.
  function renderLofi() {
    if (!lofi || renderer.running) return
    var seed = Math.floor(Math.random() * 4294967296)
    renderer.file = lofiDir + "/" + seed + ".wav"
    renderer.command = ["sh", "-c", 'mkdir -p "$1" && { ls -t "$1"/*.wav 2>/dev/null | tail -n +4 | xargs -r rm -f; } && exec "$2" "$3" "$4" "$5"',
      "sh", lofiDir, lofiBin, lofi, String(seed), renderer.file]
    renderer.running = true
  }

  function onRendered(r, file) {
    if (!r) { error = "Lo-fi: " + (renderErr.text.trim() || "render failed"); lofiStart = false; return }
    if (!lofi) return
    if (r.vibe !== lofi) return renderLofi()
    var song = { id: "lofi-" + file, path: file, title: r.name, artist: "Lo-fi · " + r.label, album: r.bpm + " BPM", albumId: "", coverArt: "", duration: r.duration, vibe: r.vibe }
    if (lofiStart) {
      var vibe = lofi
      play([song], 0, false)
      lofi = vibe
      // a replace at the same playlist position sends no playlist-pos change, so onTrack may not run
      return renderLofi()
    }
    queue = queue.concat([song])
    mpv(["loadfile", song.path, "append-play"])
  }

  // The binary prints the track as JSON only after the WAV is written. The result is handled once the process
  // has exited and its output has ended (either can come first), a turn later so `running` is false by then.
  Process {
    id: renderer
    property string file: ""
    property var out: undefined
    property bool ended: false
    function check() {
      if (out === undefined || !ended) return
      var r = null
      try { r = JSON.parse(out) } catch (e) {}
      out = undefined
      ended = false
      Qt.callLater(root.onRendered, r, file)
    }
    onExited: { ended = true; check() }
    stderr: StdioCollector { id: renderErr; waitForEnd: true }
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: { renderer.out = text; renderer.check() }
    }
  }

  // Half the pool, so a small selection still has songs left to draw.
  function recentLimit() {
    var n = 0
    for (var i = 0; i < albums.length; i++) if (!excluded[albums[i].id]) n += albums[i].songCount
    return Math.max(1, Math.min(200, Math.floor(n / 2)))
  }

  function onMpv(msg) {
    if (msg.event !== "property-change") return
    if (msg.name === "playlist-pos") onTrack(msg.data === undefined ? -1 : msg.data)
    else if (msg.name === "pause") paused = msg.data === true
    else if (msg.name === "time-pos") position = msg.data || 0
    else if (msg.name === "duration") duration = msg.data || 0
  }

  // For keybinds: omarchy-shell navidrome shuffle
  IpcHandler {
    target: "navidrome"
    function shuffle(): void { root.startShuffle() }
    function lofi(vibe: string): void { root.startLofi(vibe) }
    function toggle(): void { root.togglePause() }
    function next(): void { root.next() }
    function previous(): void { root.previous() }
    function status(): string {
      return JSON.stringify({ configured: root.configured, error: root.error, albums: root.albums.length, excluded: root.excludedCount, shuffle: root.shuffle, lofi: root.lofi, rendering: root.rendering, playlist: root.playlistId, paused: root.paused, queue: root.queue.length, index: root.index, current: root.current })
    }
  }

  Process {
    id: player
    running: root.configured
    command: ["mpv", "--idle=yes", "--no-video", "--no-terminal", "--input-ipc-server=" + root.socketPath]
    onStarted: connectTimer.restart()
    onExited: {
      ipc.connected = false
      if (root.configured) respawn.restart()
    }
  }

  Socket {
    id: ipc
    path: root.socketPath
    parser: SplitParser {
      onRead: function(line) {
        try { root.onMpv(JSON.parse(line)) } catch (e) {}
      }
    }
    onConnectedChanged: {
      if (!connected) return
      connectTimer.stop()
      var props = ["playlist-pos", "pause", "time-pos", "duration"]
      for (var i = 0; i < props.length; i++) root.mpv(["observe_property", i + 1, props[i]])
    }
  }

  // mpv creates its socket a moment after starting.
  Timer {
    id: connectTimer
    interval: 150
    repeat: true
    onTriggered: if (player.running && !ipc.connected) ipc.connected = true; else connectTimer.stop()
  }

  Timer {
    id: respawn
    interval: 3000
    onTriggered: player.running = root.configured
  }

  // --- persistence ---------------------------------------------------------

  function save() {
    configFile.setText(JSON.stringify({ server: server, user: user, salt: salt, token: token, excluded: Object.keys(excluded), lofiVibe: lofiVibe }, null, 2) + "\n")
  }

  Timer {
    id: saveTimer
    interval: 500
    onTriggered: root.save()
  }

  FileView {
    id: configFile
    path: root.configPath
    watchChanges: false
    atomicWrites: true
    printErrors: false
    onSaved: Quickshell.execDetached(["chmod", "600", root.configPath])
    onLoaded: {
      var c = {}
      try { c = JSON.parse(text()) } catch (e) {}
      root.server = c.server || ""
      root.user = c.user || ""
      root.salt = c.salt || ""
      root.token = c.token || ""
      var ex = {}
      for (var i = 0; i < (c.excluded || []).length; i++) ex[c.excluded[i]] = true
      root.excluded = ex
      root.excludedCount = Object.keys(ex).length
      root.lofiVibe = c.lofiVibe || "chill"
      root.loadAlbums()
    }
  }
}
