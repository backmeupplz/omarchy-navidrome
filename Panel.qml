import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import qs.Commons
import qs.Ui
import "Library.js" as Lib

Panel {
  id: root
  moduleName: "backmeupplz.navidrome"
  ipcTarget: "backmeupplz.navidrome"

  readonly property var nd: bar && bar.shell ? bar.shell.serviceFor("backmeupplz.navidrome") : null
  readonly property bool configured: !!nd && nd.configured
  readonly property var song: nd ? nd.current : null
  readonly property bool playing: !!song && !nd.paused
  readonly property color foreground: bar ? bar.foreground : Color.foreground
  readonly property color dim: Qt.darker(foreground, 1.55)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family

  readonly property string glyph: String.fromCodePoint(0xF075A)       // nf-md-music
  readonly property string playGlyph: String.fromCodePoint(0xF040A)
  readonly property string pauseGlyph: String.fromCodePoint(0xF03E4)

  property string query: ""
  property var openAlbum: null   // album whose tracks are shown; null shows the album list
  property var tracks: []
  property int cursor: 0

  readonly property var shown: {
    var list = nd ? nd.albums : []
    var q = query
    return q ? list.filter(function(a) { return Lib.matches(a, q) }) : list
  }
  readonly property var rows: openAlbum ? tracks : shown

  onShownChanged: if (!openAlbum) cursor = 0

  function clock(s) {
    s = Math.floor(s || 0)
    return Math.floor(s / 60) + ":" + (s % 60 < 10 ? "0" : "") + s % 60
  }

  function showAlbum(album) {
    if (!album) return
    openAlbum = album
    tracks = []
    cursor = 0
    nd.songs(album.id, function(list) { if (root.openAlbum === album) root.tracks = list })
  }

  function back() {
    var id = openAlbum.id
    openAlbum = null
    cursor = Math.max(0, shown.findIndex(function(a) { return a.id === id }))
  }

  function activate(i) {
    if (openAlbum) nd.play(tracks, i, false)
    else showAlbum(shown[i])
  }

  function toggleRow(i) {
    if (!openAlbum && shown[i]) nd.setSelected(shown[i].id, !!nd.excluded[shown[i].id])
  }

  function move(d) { cursor = Math.max(0, Math.min(rows.length - 1, cursor + d)) }

  function connect() { nd.login(serverField.text, userField.text, passField.text) }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    dimmed: !root.configured
    text: root.glyph
    tooltipText: "Navidrome"
    onPressed: function(b) {
      if (b === Qt.MiddleButton && root.nd) root.nd.togglePause()
      else root.toggle()
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: !root.configured ? (serverField.text ? passField : serverField) : keys
    contentWidth: panel.fittedContentWidth(Style.space(440))
    contentHeight: root.configured ? panel.fittedContentHeight(Style.space(640)) : panel.fittedContentHeight(login.implicitHeight)

    Item {
      id: keys
      anchors.fill: parent
      focus: true

      Keys.onPressed: function(e) {
        if (!root.configured) {
          if (e.key === Qt.Key_Escape) { root.close(); e.accepted = true }
          return
        }
        var t = e.text
        e.accepted = true
        if (e.key === Qt.Key_Escape) root.openAlbum ? root.back() : root.close()
        else if (e.key === Qt.Key_Down || t === "j") root.move(1)
        else if (e.key === Qt.Key_Up || t === "k") root.move(-1)
        else if (e.key === Qt.Key_PageDown) root.move(10)
        else if (e.key === Qt.Key_PageUp) root.move(-10)
        else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Right || t === "l") root.activate(root.cursor)
        else if ((e.key === Qt.Key_Left || e.key === Qt.Key_Backspace || t === "h") && root.openAlbum) root.back()
        else if (e.key === Qt.Key_Space) root.toggleRow(root.cursor)
        else if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) root.switchPanel(e.key === Qt.Key_Backtab ? -1 : 1)
        else if (t === "/" && !root.openAlbum) search.forceActiveFocus()
        else if (t === "s") root.nd.startShuffle()
        else if (t === "p") root.nd.togglePause()
        else if (t === "n") root.nd.next()
        else if (t === "b") root.nd.previous()
        else if (t === "r") root.nd.loadAlbums()
        else e.accepted = false
      }

      // --- sign in ---------------------------------------------------------

      ColumnLayout {
        id: login
        visible: !root.configured
        width: parent.width
        spacing: Style.space(8)

        PanelHero {
          Layout.fillWidth: true
          title: "Navidrome"
          meta: root.nd && root.nd.error ? root.nd.error : "Connect to your server"
          foreground: root.foreground
          fontFamily: root.fontFamily
          iconComponent: Component {
            Text { text: root.glyph; color: root.foreground; font.family: root.fontFamily; font.pixelSize: Style.font.display }
          }
        }

        TextField {
          id: serverField
          Layout.fillWidth: true
          placeholderText: "https://music.example.com"
          text: root.nd ? root.nd.server : ""
          foreground: root.foreground
          onAccepted: userField.forceActiveFocus()
          Keys.onEscapePressed: root.close()
        }

        TextField {
          id: userField
          Layout.fillWidth: true
          placeholderText: "Username"
          text: root.nd ? root.nd.user : ""
          foreground: root.foreground
          onAccepted: passField.forceActiveFocus()
          Keys.onEscapePressed: root.close()
        }

        TextField {
          id: passField
          Layout.fillWidth: true
          placeholderText: "Password"
          password: true
          foreground: root.foreground
          onAccepted: root.connect()
          Keys.onEscapePressed: root.close()
        }

        Button {
          Layout.alignment: Qt.AlignRight
          text: root.nd && root.nd.loading ? "Connecting…" : "Connect"
          bordered: true
          foreground: root.foreground
          enabled: serverField.text !== "" && userField.text !== "" && passField.text !== ""
          onClicked: root.connect()
        }
      }

      // --- player + library ------------------------------------------------

      ColumnLayout {
        visible: root.configured
        anchors.fill: parent
        spacing: Style.space(10)

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(12)

          Cover {
            size: Style.space(52)
            coverId: root.song ? root.song.coverArt : ""
          }

          ColumnLayout {
            Layout.fillWidth: true
            spacing: Style.space(2)
            Text {
              Layout.fillWidth: true
              text: root.song ? root.song.title : (root.nd && root.nd.error ? root.nd.error : "Nothing playing")
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.title
              font.bold: true
              elide: Text.ElideRight
            }
            Text {
              Layout.fillWidth: true
              text: root.song ? root.song.artist + " · " + root.song.album : "Pick an album, or shuffle your selection"
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              elide: Text.ElideRight
            }
          }

          PanelActionButton {
            iconText: String.fromCodePoint(0xF04AE)
            foreground: root.foreground
            enabled: !!root.song
            onClicked: root.nd.previous()
          }
          PanelActionButton {
            iconText: root.playing ? root.pauseGlyph : root.playGlyph
            foreground: root.foreground
            enabled: !!root.song
            onClicked: root.nd.togglePause()
          }
          PanelActionButton {
            iconText: String.fromCodePoint(0xF04AD)
            foreground: root.foreground
            enabled: !!root.song
            onClicked: root.nd.next()
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: !!root.song
          spacing: Style.space(8)
          Text { text: root.clock(root.nd ? root.nd.position : 0); color: root.dim; font.family: root.fontFamily; font.pixelSize: Style.font.caption }
          PanelSlider {
            Layout.fillWidth: true
            bar: root.bar
            minimum: 0
            maximum: Math.max(1, root.nd ? root.nd.duration : 1)
            step: 1
            value: root.nd ? root.nd.position : 0
            onReleased: function(v) { root.nd.seek(v) }
          }
          Text { text: root.clock(root.nd ? root.nd.duration : 0); color: root.dim; font.family: root.fontFamily; font.pixelSize: Style.font.caption }
        }

        PanelSeparator { Layout.fillWidth: true; foreground: root.foreground }

        RowLayout {
          Layout.fillWidth: true
          visible: !root.openAlbum
          spacing: Style.space(8)

          TextField {
            id: search
            Layout.fillWidth: true
            placeholderText: "Search albums and artists  ( / )"
            foreground: root.foreground
            onTextChanged: root.query = text
            Keys.onReturnPressed: keys.forceActiveFocus()
            Keys.onEnterPressed: keys.forceActiveFocus()
            Keys.onDownPressed: keys.forceActiveFocus()
            Keys.onEscapePressed: text ? text = "" : keys.forceActiveFocus()
          }

          Button {
            text: root.nd && root.nd.picking ? "Shuffling…" : "Shuffle"
            iconText: String.fromCodePoint(0xF049D)
            bordered: true
            foreground: root.foreground
            active: !!root.nd && root.nd.shuffle
            tooltipText: "Shuffle songs from the checked albums  ( s )"
            onClicked: root.nd.startShuffle()
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: !root.openAlbum
          spacing: Style.space(6)

          Text {
            Layout.fillWidth: true
            readonly property int total: root.nd ? root.nd.albums.length : 0
            text: root.nd && root.nd.loading ? "Loading library…"
              : Math.max(0, total - root.nd.excludedCount).toLocaleString(Qt.locale(), "f", 0) + " of " + total.toLocaleString(Qt.locale(), "f", 0) + " albums in shuffle"
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            elide: Text.ElideRight
          }
          Button { text: "All"; foreground: root.foreground; onClicked: root.nd.selectAll(true) }
          Button { text: "None"; foreground: root.foreground; onClicked: root.nd.selectAll(false) }
          PanelActionButton {
            iconText: String.fromCodePoint(0xF0343)
            tooltipText: "Sign out"
            foreground: root.foreground
            onClicked: root.nd.logout()
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: !!root.openAlbum
          spacing: Style.space(8)

          PanelActionButton {
            iconText: String.fromCodePoint(0xF004D)
            tooltipText: "Back  ( Esc )"
            foreground: root.foreground
            onClicked: root.back()
          }
          ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            Text {
              Layout.fillWidth: true
              text: root.openAlbum ? root.openAlbum.name : ""
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.subtitle
              font.bold: true
              elide: Text.ElideRight
            }
            Text {
              Layout.fillWidth: true
              text: root.openAlbum ? root.openAlbum.artist + (root.openAlbum.year ? " · " + root.openAlbum.year : "") : ""
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              elide: Text.ElideRight
            }
          }
          Button {
            text: "Play"
            iconText: root.playGlyph
            bordered: true
            foreground: root.foreground
            onClicked: root.nd.play(root.tracks, 0, false)
          }
        }

        ListView {
          id: list
          Layout.fillWidth: true
          Layout.fillHeight: true
          clip: true
          reuseItems: true
          boundsBehavior: Flickable.StopAtBounds
          model: root.rows
          currentIndex: root.cursor
          onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
          ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

          delegate: CursorSurface {
            id: row
            required property var modelData
            required property int index
            readonly property bool isAlbum: !root.openAlbum
            readonly property bool checked: isAlbum && !!root.nd && !root.nd.excluded[modelData.id]

            width: ListView.view.width
            height: Style.space(40)
            foreground: root.foreground
            hasCursor: index === root.cursor
            current: !isAlbum && !!root.song && root.song.id === modelData.id

            MouseArea {
              anchors.fill: parent
              hoverEnabled: true
              cursorShape: Qt.PointingHandCursor
              onEntered: root.cursor = row.index
              onClicked: root.activate(row.index)
            }

            RowLayout {
              anchors.fill: parent
              anchors.leftMargin: Style.space(8)
              anchors.rightMargin: Style.space(10)
              spacing: Style.space(10)

              Text {
                Layout.preferredWidth: Style.space(20)
                horizontalAlignment: Text.AlignHCenter
                text: row.isAlbum ? String.fromCodePoint(row.checked ? 0xF0132 : 0xF0131) : (row.modelData.track || row.index + 1)
                color: row.isAlbum && row.checked ? root.foreground : root.dim
                font.family: root.fontFamily
                font.pixelSize: row.isAlbum ? Style.font.icon : Style.font.caption

                MouseArea {
                  anchors.fill: parent
                  anchors.margins: -Style.space(6)
                  enabled: row.isAlbum
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.toggleRow(row.index)
                }
              }

              Cover {
                visible: row.isAlbum
                size: Style.space(30)
                coverId: row.isAlbum ? row.modelData.coverArt : ""
              }

              ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text {
                  Layout.fillWidth: true
                  text: row.isAlbum ? row.modelData.name : row.modelData.title
                  color: root.foreground
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.body
                  font.bold: row.current
                  elide: Text.ElideRight
                }
                Text {
                  Layout.fillWidth: true
                  text: row.modelData.artist + (row.isAlbum && row.modelData.year ? " · " + row.modelData.year : "")
                  color: root.dim
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                  elide: Text.ElideRight
                }
              }

              Text {
                text: row.isAlbum ? row.modelData.songCount : root.clock(row.modelData.duration)
                color: root.dim
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
            }
          }
        }
      }
    }
  }

  component Cover: Rectangle {
    property string coverId: ""
    property real size: 40
    Layout.preferredWidth: size
    Layout.preferredHeight: size
    radius: Style.cornerRadius
    color: Style.selectedFillFor(root.foreground, Color.accent)
    clip: true

    Text {
      anchors.centerIn: parent
      visible: img.status !== Image.Ready
      text: root.glyph
      color: root.dim
      font.family: root.fontFamily
      font.pixelSize: parent.size * 0.45
    }

    Image {
      id: img
      anchors.fill: parent
      source: root.nd && parent.coverId ? root.nd.coverUrl(parent.coverId, 128) : ""
      sourceSize.width: 128
      sourceSize.height: 128
      fillMode: Image.PreserveAspectCrop
      asynchronous: true
    }
  }
}
