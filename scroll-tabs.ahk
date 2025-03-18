;; =============================================
;; scroll-wheel-tab-switching ~e4zyphil
;; scroll-tabs.ahk
;; =============================================

;; CONFIG
;; =============================================
; Add all suitable programs here :) I tried sorting them alphabetically
global ALLOWED_PROGRAMS := [
  "brave.exe",
  "Code.exe",
  "chrome.exe",
  "chromium.exe",
  "explorer.exe",
  "firefox.exe",
  "opera.exe",
  "opera_gx.exe",
  "msedge.exe",
  "WindowsTerminal.exe",
]
; If the active window should be returned after scrolling to change tab in inactive window
global ENABLE_WINDOW_RETURN := false
global DEBUG := true

;; GENERAL & GLOBALS
;; =============================================
#Requires AutoHotkey v2
global NeedReturn := false
CoordMode( "Mouse", "Screen" )

;; HELPER FUNCTIONS
;; =============================================
ProcessInALLOWED_PROGRAMS( processToFind ) {
  global ALLOWED_PROGRAMS
  for program in ALLOWED_PROGRAMS {
    if ( StrLower( processToFind ) = StrLower( program ) ) {
      return true
    }
  }
  return false
}
GetRelativeMouseY( &relativeY ) {
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  ; Retrieve the monitor handle for the current mouse position (MONITOR_DEFAULTTONEAREST = 2)
  local monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" )

  ; Allocate a buffer for the MONITORINFO structure (40 bytes)
  local monitorInfo := Buffer( 40, 0 )
  ; Set the structure size (dwSize = 40)
  NumPut( "UInt", 40, monitorInfo, 0 )

  ; Retrieve monitorInfo
  if !DllCall( "GetMonitorInfo", "Ptr", monitorHandle, "Ptr", monitorInfo ) {
    LogError( "error", "GetMonitorInfo failed for handle " monitorHandle )
    return false
  }

  ; Die MONITORINFO-Struktur ist folgendermaßen aufgebaut:
  ; dwSize (4 Byte) an Offset 0,
  ; rcMonitor (RECT) ab Offset 4: left (4), top (8), right (12), bottom (16)

  ;;; local monitorLeft   := NumGet(monitorInfo, 4, "Int")
  local monitorTop := NumGet( monitorInfo, 8, "Int" )
  ;;; local monitorRight  := NumGet(monitorInfo, 12, "Int")
  ;;; local monitorBottom := NumGet(monitorInfo, 16, "Int")
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
TrackScrollActivity() {
  global lastScrollTime
  lastScrollTime := A_TickCount
  MsgBox( lastScrollTime )
  SetTimer( CheckScrollInactivity, -2000 )  ; Timer startet EINMALIG nach 2 Sekunden
}
CheckScrollInactivity() {
  global lastScrollTime, previousActiveWindow, NeedReturn

  if ( A_TickCount - lastScrollTime > 2000 ) {  ; 2000 ms = 2 Sekunden
    MsgBox( "Kein Scrollen für 2 Sekunden!" )
    ActivateWindow( previousActiveWindow )
    NeedReturn := false
    SetTimer( CheckScrollInactivity, 0 )  ; Timer stoppen
  }
}
LogError( level, message ) {
  formattedTime := "[" FormatTime( A_Now, "yyyy-MM-dd HH:mm:ss" ) "] "
  formattedLevel := "[" StrUpper( level ) "] "
  FileAppend( formattedTime formattedLevel message "`n", A_ScriptDir "\error.log" )
}

;; MAIN
;; =============================================
~$WheelDown::
~$WheelUp::
{
  global ENABLE_WINDOW_RETURN, previousActiveWindow, NeedReturn
  MouseGetPos(, , &mouseWindowId )
  local processName := WinGetProcessName( mouseWindowId )
  if ( !ProcessInALLOWED_PROGRAMS( processName ) ) {
    return
  }
  GetRelativeMouseY( &relativeY )
  if ( relativeY > 50 ) {
    return
  }
  if !( NeedReturn ) {
    previousActiveWindow := WinGetID( "A" )
  }
  if ( !IsMouseOverActiveWindow( mouseWindowId, previousActiveWindow ) ) {
    ActivateWindow( mouseWindowId )
    NeedReturn := true
  }
  if ( A_ThisHotkey = "~$WheelDown" ) {
    Send( "^{Tab}" )
  } else {
    Send( "^+{Tab}" )
  }
  ; (optional) Return to original window
  if ( ENABLE_WINDOW_RETURN && NeedReturn ) {
    TrackScrollActivity()
  }
}

;; DEBUG
;; =============================================
class DebugGUI {
  __New() {
    this.gui := Gui( "+Resize +MinSize380x220 -MaximizeBox" )
    this.gui.Title := "Debug"
    this.gui.BackColor := "1C1C1C"
    this.gui.SetFont( "s14 q5 cffffff", "Segoe UI" )
    SetDarkMode( this.gui.Hwnd )

    this.layout := { startX: 20, startY: 10, lineSpacing: 25, widthLabel: 290, widthValue: 200 }

    this.fields := [
      { Key: "ahkVersion", Label: "AHK Version:" },
      { Key: "ENABLE_WINDOW_RETURN", Label: "ENABLE_WINDOW_RETURN:" },
      { Key: "mouseX", Label: "mouseX:" },
      { Key: "mouseY", Label: "mouseY:" },
      { Key: "relativeY", Label: "relativeY:" },
      { Key: "mouseWindowId", Label: "mouseWindowId:" },
      { Key: "processName", Label: "processName:" },
      { Key: "ProcessInALLOWED_PROGRAMS", Label: "processInALLOWED_PROGRAMS:" },
    ]

    this.guiFields := Map()
    this.CreateGUIElements()
    this.gui.OnEvent( "Size", this.OnResize.Bind( this ) )

    ; ✅ Store a bound function reference to ensure SetTimer resets correctly
    this.reloadGuiBound := this.ReloadGUI.Bind( this )
  }

  CreateGUIElements() {
    for each, field in this.fields {
      yPos := this.layout.startY + this.layout.lineSpacing * ( A_Index - 1 )
      this.gui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans", this.layout.startX, yPos, this.layout.widthLabel ), field.Label )

      this.guiFields[ field.Key ] := this.gui.Add(
        ( field.Key = "processName" ) ? "Edit" : "Text",
        Format( "x{} y{} w{} {} Right",
          this.layout.startX + this.layout.widthLabel + 10, yPos, this.layout.widthValue,
          ( field.Key = "processName" ) ? "+ReadOnly -E0x200" : "BackgroundTrans"
        )
      )

      if ( field.Key = "processName" ) {
        this.guiFields[ field.Key ].Opt( "+Background" this.gui.BackColor )
      }
    }
  }

  OnResize( GuiObj, MinMax, Width, Height ) {
    if ( MinMax = -1 || MinMax = 1 ) {
      return ; Ignore minimize/maximize
    }

    for each, field in this.fields {
      this.guiFields[ field.Key ].Move( Width - this.layout.widthValue - 20, this.layout.startY + this.layout.lineSpacing * ( A_Index - 1 ) )
    }

    ; ✅ Reset the timer correctly by using the stored bound function reference
    SetTimer( this.reloadGuiBound, -500 )  ; If called again, it resets to 500ms
  }

  UpdateDebugGUI() {
    MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
    local processName := WinGetProcessName( mouseWindowId )
    GetRelativeMouseY( &relativeY )

    this.guiFields[ "ahkVersion" ].Text := A_AhkVersion
    this.guiFields[ "ENABLE_WINDOW_RETURN" ].Text := ENABLE_WINDOW_RETURN ? "true" : "false"
    this.guiFields[ "mouseX" ].Text := mouseX
    this.guiFields[ "mouseY" ].Text := mouseY
    this.guiFields[ "relativeY" ].Text := relativeY
    this.guiFields[ "mouseWindowId" ].Text := mouseWindowId
    this.guiFields[ "processName" ].Text := processName
    this.guiFields[ "ProcessInALLOWED_PROGRAMS" ].Text := ProcessInALLOWED_PROGRAMS( processName ) ? "true" : "false"

    this.guiFields[ "processName" ].SetFont( "Bold" )

    this.gui.Show( Format( "x{} y{} w380 h220", mouseX - 190, mouseY ) )
    this.guiFields[ "ProcessInALLOWED_PROGRAMS" ].Focus()
  }

  ReloadGUI() {
    this.gui.Hide()
    this.guiFields[ "ProcessInALLOWED_PROGRAMS" ].Focus()
    this.gui.Show()
  }
}

SetDarkMode( hwnd ) {
  static DWMWA_USE_IMMERSIVE_DARK_MODE := 20
  versionParts := StrSplit( A_OSVersion, "." )
  if ( versionParts[ 1 ] + 0 >= 10 && versionParts[ 3 ] + 0 >= 17763 ) {
    DllCall( "dwmapi\DwmSetWindowAttribute", "ptr", hwnd, "int", DWMWA_USE_IMMERSIVE_DARK_MODE, "int*", 1, "int", 4 )
  }
}

if ( DEBUG ) {
  DebugInstance := DebugGUI()
  $F12:: {
    DebugInstance.UpdateDebugGUI()
  }
}

return