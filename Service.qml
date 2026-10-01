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

  // --- Subsonic ------------------------------------------------------------

  function url(method, params) {
    var q = "u=" + encodeURIComponent(user) + "&t=" + token + "&s=" + salt + "&v=1.16.1&c=omarchy-navidrome&f=json"
    for (var k in params) q += "&" + k + "=" + encodeURIComponent(params[k])
    return server + "/rest/" + method + "?" + q
  }

  function coverUrl(id, size) {
    return id && configured ? url("getCoverArt", { id: id, size: size }) : ""
  }

  function api(method, params, done) {
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
    xhr.open("GET", url(method, params))
    xhr.send()
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
        else { albums = acc; loading = false }
      })
    }
    page(0)
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

  function streamUrl(song) { return url("stream", { id: song.id }) }

  function mpv(args) {
    if (!ipc.connected) return
    ipc.write(JSON.stringify({ command: args }) + "\n")
    ipc.flush()
  }

  function play(list, start, isShuffle) {
    if (!list.length) return
    shuffle = !!isShuffle
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
  function stop() { shuffle = false; queue = []; mpv(["stop"]) }

  function onTrack(pos) {
    index = pos
    position = 0
    if (!current) return
    recent = recent.concat([current.id]).slice(-recentLimit())
    // ponytail: scrobbles at track start; move to the halfway mark if play counts matter.
    api("scrobble", { id: current.id, submission: true }, function() {})
    if (shuffle && index === queue.length - 1) appendRandom()
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
    function toggle(): void { root.togglePause() }
    function next(): void { root.next() }
    function previous(): void { root.previous() }
    function status(): string {
      return JSON.stringify({ configured: root.configured, error: root.error, albums: root.albums.length, excluded: root.excludedCount, shuffle: root.shuffle, paused: root.paused, queue: root.queue.length, index: root.index, current: root.current })
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
    configFile.setText(JSON.stringify({ server: server, user: user, salt: salt, token: token, excluded: Object.keys(excluded) }, null, 2) + "\n")
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
      root.loadAlbums()
    }
  }
}
