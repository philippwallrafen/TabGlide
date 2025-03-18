;; Scroll wheel tab-switching ~e4zyphil
#Requires AutoHotkey v2

; Add all suitable programs here :) I tried sorting them alphabetically
global allowedPrograms := Array( "brave.exe", "chrome.exe", "explorer.exe", "msedge.exe", )
global ReturnToOriginalWindow := true
global NeedReturn := false

CoordMode( "Mouse", "Screen" )
; Helper functions
GetMouseInfo( &mouseX, &mouseY, &mouseWindowId ) {
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
}
ProcessIsInAllowedPrograms( processToFind ) {
  global allowedPrograms
  for program in allowedPrograms {
    if ( StrLower( program ) = StrLower( processToFind ) ) {
      return true
    }
  }
  return false
}
GetRelativeMouseY( &relativeY ) {
  GetMouseInfo( &mouseX, &mouseY, &mouseWindowId )
  ; Hole den Monitor-Handle für den aktuellen Punkt (MONITOR_DEFAULTTONEAREST = 2)
  local hMon := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" )

  ; Erstelle einen Puffer für die MONITORINFO-Struktur (40 Byte)
  monitorInfo := Buffer( 40, 0 )
  ; Setze die Größe der Struktur (dwSize = 40)
  NumPut( "UInt", 40, monitorInfo, 0 )  ; Setze dwSize = 40

  ; Rufe GetMonitorInfo auf, um die Monitorinformationen zu erhalten
  local getMonInfoResult := DllCall( "GetMonitorInfo", "Ptr", hMon, "Ptr", monitorInfo )

  ; Die MONITORINFO-Struktur ist folgendermaßen aufgebaut:
  ; dwSize (4 Byte) an Offset 0,
  ; rcMonitor (RECT) ab Offset 4: left (4), top (8), right (12), bottom (16)

  ; local monitorLeft   := NumGet(monitorInfo, 4, "Int")
  local monitorTop := NumGet( monitorInfo, 8, "Int" )
  ; local monitorRight  := NumGet(monitorInfo, 12, "Int")
  ; local monitorBottom := NumGet(monitorInfo, 16, "Int")

  return relativeY := mouseY - monitorTop
}
IsMouseOverActiveWindow( mouseWindowId, previousActiveWindow ) {
  return ( mouseWindowId = previousActiveWindow )
}
ActivateWindow( windowId ) {
  if ( windowId ) {
    WinActivate( windowId )
  }
}
TrackScrollActivity( ) {
  ; static lastScrollTime := 0  ; Speichert den letzten Scroll-Zeitpunkt
  global lastScrollTime  ; Scroll-Zeit aktualisieren
  lastScrollTime := A_TickCount  ; Scroll-Zeit aktualisieren
  MsgBox( lastScrollTime )
  SetTimer( CheckScrollInactivity, -2000 )  ; Timer startet EINMALIG nach 2 Sekunden
}
CheckScrollInactivity( ) {
  ; static lastScrollTime := 0  ; Speichert den Zeitpunkt des letzten Scrollens
  global lastScrollTime, previousActiveWindow, NeedReturn

  if ( A_TickCount - lastScrollTime > 2000 ) {  ; 2000 ms = 2 Sekunden
    MsgBox( "Kein Scrollen für 2 Sekunden!" )  ; Nachricht anzeigen
    ActivateWindow( previousActiveWindow )
    NeedReturn := false
    SetTimer( CheckScrollInactivity, 0 )  ; Timer stoppen
  }
}

~$WheelDown::
~$WheelUp::
{
  GetMouseInfo( &mouseX, &mouseY, &mouseWindowId )
  local processName := WinGetProcessName( mouseWindowId )
  if ( !ProcessIsInAllowedPrograms( processName ) ) {
    return
  }
  GetRelativeMouseY( &relativeY )
  if ( relativeY > 50 ) {
    return
  }
  ; global previousActiveWindow
  if !( NeedReturn ) {
    global previousActiveWindow := WinGetID( "A" )
  }
  if ( !IsMouseOverActiveWindow( mouseWindowId, previousActiveWindow ) ) {
    ActivateWindow( mouseWindowId )
    global ReturnToOriginalWindow
    global NeedReturn := true
  }
  ; }
  if ( A_ThisHotkey = "~$WheelDown" ) {
    Send( "^{Tab}" )
  } else {
    Send( "^+{Tab}" )
  }
  ; (optional) Return to original window
  ; if ( ReturnToOriginalWindow && mouseWindowId != previousActiveWindow ) {
  if ( ReturnToOriginalWindow && NeedReturn ) {
    TrackScrollActivity( )
    ; MsgBox( "going back" )
    ; Sleep 5
    ; ActivateWindow( previousActiveWindow )
  }
}

$F12:: ; debug Key to show processName
{
  GetMouseInfo( &mouseX, &mouseY, &mouseWindowId )
  local processName := WinGetProcessName( mouseWindowId )
  GetRelativeMouseY( &relativeY )

  ; ✅ Update GUI values dynamically
  guiFields[ "AhkVersion" ].Text := A_AhkVersion
  guiFields[ "MouseX" ].Text := mouseX
  guiFields[ "MouseY" ].Text := mouseY
  guiFields[ "RelativeY" ].Text := relativeY
  guiFields[ "mouseWindowId" ].Text := mouseWindowId
  guiFields[ "ProcessName" ].Text := processName
  guiFields[ "ProcessIsInAllowedPrograms" ].Text := ProcessIsInAllowedPrograms( processName ) ? "true" : "false"

  guiFields[ "ProcessName" ].SetFont( "Bold" )

  MyGui.Show( Format( "x{} y{}", mouseX - 170, mouseY ) )
}

; Erstellen einer neuen GUI
MyGui := Gui( )
MyGui.BackColor := "FFFFFF"
MyGui.Opt( "+MinSize600x300" )
MyGui.SetFont( "s14 q5", "Segoe UI" )

startX := 20
startValueX := 0
startY := 10
lineSpacing := 25
widthLabel := 150
widthValue := 180

; Array mit Label-Texten und entsprechenden Schlüsseln
fields := [
  { Key: "AhkVersion", Label: "AHK Version:" },
  { Key: "MouseX", Label: "mouseX:" },
  { Key: "MouseY", Label: "mouseY:" },
  { Key: "RelativeY", Label: "relativeY:" },
  { Key: "mouseWindowId", Label: "mouseWindowId:" },
  { Key: "ProcessName", Label: "processName:" },
  { Key: "ProcessIsInAllowedPrograms", Label: "ProcessIsInAllowedPrograms:" },
]

; Objekt zum Speichern der GUI-Feld-Referenzen
global guiFields := Map( )

for each, field in fields {
  yPos := startY + lineSpacing * ( A_Index - 1 )
  MyGui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans", startX, yPos, widthLabel ), field.Label )
  guiFields[ field.Key ] := MyGui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans Right", startX + widthLabel + 10, yPos, widthValue ) )
}

MyGui.Title := "Debug"

return