;; =============================================
;; scroll-wheel-tab-switching ~e4zyphil
;; scroll-tabs.ahk
;; =============================================

;; CONFIG
;; =============================================
; Add all suitable programs here :) I tried sorting them alphabetically
global ALLOWED_PROGRAMS := Map(
  "brave.exe", true,
  "Code.exe", true,
  "chrome.exe", true,
  "chromium.exe", true,
  "explorer.exe", true,
  "firefox.exe", true,
  "opera.exe", true,
  "opera_gx.exe", true,
  "WindowsTerminal.exe", true,
  "msedge.exe", true
)
global ENABLE_WINDOW_RETURN := true     ; If you want to return after scrolling on an inactive window
global RETURN_AFTER_MS := 700           ; Fine tune - Return to the previous active window after _ in ms
global TOP_REGION_PIXEL_LIMIT := 50     ; Fine tune - how forgiving the Y coordinate is
global DEBUG := false                   ; Debug Mode

;; GENERAL & START-UP
;; =============================================
#Requires AutoHotkey v2
CoordMode( "Mouse", "Screen" )
global awaitingReturn := false
global returnWindow := ""
if ( DEBUG ) {
  DebugGUI := InitializeDebugGUI()
  $F12:: {                              ; Define your debug hotkey here !
    UpdateDebugGUI( DebugGUI )
  }
}
; Convert ALLOWED_PROGRAMS to lowercase
lowercasedMap := Map()
for program, value in ALLOWED_PROGRAMS {
  lowercasedMap[ StrLower( program ) ] := value
}
ALLOWED_PROGRAMS := lowercasedMap

;; HELPER FUNCTIONS
;; =============================================
GetActiveWindow() {
  try {
    return WinGetID( "A" )
  } catch {
    Log( "warning", "No active window found." )
    return ""
  }
}
GetRelativeMouseY() {
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  ; Retrieve monitorHandle per Dll for the current mouse position (MONITOR_DEFAULTTONEAREST = 2)
  local monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" )
  local monitorInfo := Buffer( 40, 0 )    ; Allocate a buffer for the MONITORINFO structure (40 bytes)
  NumPut( "UInt", 40, monitorInfo, 0 )    ; Set the structure size (dwSize = 40)
  ; Retrieve monitorInfo per Dll
  if !DllCall( "GetMonitorInfo", "Ptr", monitorHandle, "Ptr", monitorInfo ) {
    Log( "error", "GetMonitorInfo failed for handle " monitorHandle )
    return false
  }
  ; The MONITORINFO structure is structured as follows:
  ; dwSize (4 bytes) at offset 0,
  ; rcMonitor (RECT) from offset 4: left (4), top (8), right (12), bottom (16)

  ; local monitorLeft   := NumGet(monitorInfo, 4, "Int")
  local monitorTop := NumGet( monitorInfo, 8, "Int" )
  ; local monitorRight  := NumGet(monitorInfo, 12, "Int")
  ; local monitorBottom := NumGet(monitorInfo, 16, "Int")

  return relativeToMonitorY := mouseY - monitorTop
}
TrackScrollActivity() {
  ;;; global RETURN_AFTER_MS
  SetTimer( AfterTimerEnds, -RETURN_AFTER_MS )
}
AfterTimerEnds() {
  global awaitingReturn, returnWindow
  if ( awaitingReturn ) {
    WinActivate( returnWindow )
    awaitingReturn := false
  }
}
Log( level, message ) {
  if ( DEBUG ) {
    formattedTime := "[" FormatTime( A_Now, "yyyy-MM-dd HH:mm:ss" ) "] "
    formattedLevel := "[" StrUpper( level ) "] "
    FileAppend( formattedTime formattedLevel message "`n", A_ScriptDir "\error.log" )
  }
}

;; MAIN
;; =============================================
~$WheelDown::
~$WheelUp::
{
  ;;; global TOP_REGION_PIXEL_LIMIT, ENABLE_WINDOW_RETURN
  global returnWindow, awaitingReturn
  MouseGetPos(, , &mouseWindowId )
  local processName := WinGetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) )
  if ( !processInMap ) {
    return
  }
  local relativeToMonitorY := GetRelativeMouseY()
  if ( relativeToMonitorY > TOP_REGION_PIXEL_LIMIT ) {
    return
  }
  local activeWindow := GetActiveWindow(), isDifferentWindow := mouseWindowId != activeWindow
  ; (optional) Return to original window
  if ( ENABLE_WINDOW_RETURN ) {
    TrackScrollActivity()
    if ( isDifferentWindow && !awaitingReturn ) {
      returnWindow := activeWindow
      awaitingReturn := true
    }
  }
  if ( isDifferentWindow ) {
    WinActivate( mouseWindowId )
  }
  if ( A_ThisHotkey = "~$WheelDown" ) {
    Send( "^{Tab}" )
  } else {
    Send( "^+{Tab}" )
  }
}

;; DEBUG-GUI: Helper functions
;; =============================================
DarkModeTitleBar( Hwnd ) {
  local osVersion := StrSplit( A_OSVersion, "." ), major := osVersion[ 1 ] + 0, build := osVersion[ 3 ] + 0
  static DWMWA_USE_IMMERSIVE_DARK_MODE := 20
  if ( major >= 10 && build >= 17763 ) {
    DllCall( "dwmapi\DwmSetWindowAttribute", "ptr", Hwnd, "int", DWMWA_USE_IMMERSIVE_DARK_MODE, "int*", 1, "int", 4 )
  }
}
OnResizeKeepTextRightAligned( DebugGUI, GuiObj, MinMax, Width, Height ) {
  if ( MinMax = -1 || MinMax = 1 ) {
    return    ; Ignore minimize/maximize
  }
  local guiFields := DebugGUI.guiFields, layout := DebugGUI.layout, fields := DebugGUI.fields
  for field in fields {
    guiFields[ field.Key ].Move( Width - layout.widthValue - 20, layout.startY + layout.lineSpacing * ( A_Index - 1 ) )
  }
  SetTimer( DebugGUI.reloadGuiBound, -500 )
}
UpdateDebugGUI( DebugGUI ) {
  ;;; global ENABLE_WINDOW_RETURN, awaitingReturn
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  local guiFields := DebugGUI.guiFields, processName := WinGetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) ), activeWindow := WinGetID( "A" ), isDifferentWindow := mouseWindowId != activeWindow, monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" ), relativeToMonitorY := GetRelativeMouseY()

  guiFields[ "ahkVersion" ].Text := A_AhkVersion
  guiFields[ "isAdmin" ].Text := A_IsAdmin ? "true" : "false"
  guiFields[ "mouseX" ].Text := mouseX
  guiFields[ "monitorHandle" ].Text := monitorHandle
  guiFields[ "mouseY" ].Text := mouseY
  guiFields[ "relativeToMonitorY" ].Text := relativeToMonitorY
  guiFields[ "processName" ].Text := processName
  guiFields[ "processInMap" ].Text := processInMap ? "true" : "false"
  guiFields[ "isDifferentWindow" ].Text := isDifferentWindow ? "true" : "false"
  guiFields[ "ENABLE_WINDOW_RETURN" ].Text := ENABLE_WINDOW_RETURN ? "true" : "false"
  guiFields[ "returnWindowName" ].Text := WinGetProcessName( mouseWindowId )
  guiFields[ "awaitingReturn" ].Text := awaitingReturn ? "true" : "false"

  guiFields[ "processName" ].SetFont( "Bold" )
  DebugGUI.gui.Show( Format( "x{} y{} w380", mouseX - 190, mouseY + 10 ) )
  guiFields[ "processInMap" ].Focus()
}
ReloadGUI( DebugGUI ) {
  DebugGUI.gui.Hide()
  DebugGUI.guiFields[ "processInMap" ].Focus()
  DebugGUI.gui.Show()
}

;; DEBUG-GUI: Create elements loop
;; =============================================
CreateGUIElements( DebugGUI ) {
  local gui := DebugGUI.gui, fields := DebugGUI.fields, layout := DebugGUI.layout, guiFields := DebugGUI.guiFields, xPos := layout.startX + layout.widthLabel + 10, yPos := layout.startY + layout.lineSpacing * ( A_Index - 1 ), wValue := layout.widthValue
  for field in fields {
    ; Left side
    gui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans", layout.startX, layout.startY + layout.lineSpacing * ( A_Index - 1 ), layout.widthLabel ), field.Label )
    ; Right side
    if ( field.Key = "processName" ) {
      guiFields[ field.Key ] := gui.Add( "Edit", Format( "x{} y{} w{} +ReadOnly -E0x200 Right", xPos, yPos, wValue ) )
    } else {
      guiFields[ field.Key ] := gui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans Right", xPos, yPos, wValue ) )
    }
    if ( field.Key = "processName" ) {
      guiFields[ field.Key ].Opt( "+Background" gui.BackColor )
    }
  }
}
;; DEBUG-GUI: Setup
;; =============================================
InitializeDebugGUI() {
  local DebugGUI := {
    gui: Gui( "+Resize +MinSize380x330 -MaximizeBox" ),
    layout: { startX: 20, startY: 10, lineSpacing: 25, widthLabel: 290, widthValue: 200 },
    guiFields: Map(),
    fields: []
  }

  DebugGUI.gui.Title := "Debug"
  DebugGUI.gui.BackColor := "1C1C1C"
  DebugGUI.gui.SetFont( "s14 q5 cffffff", "Segoe UI" )
  DarkModeTitleBar( DebugGUI.gui.Hwnd )

  debugKeys := [
    "ahkVersion",
    "isAdmin",
    "mouseX",
    "monitorHandle",
    "mouseY",
    "relativeToMonitorY",
    "processName",
    "processInMap",
    "isDifferentWindow",
    "ENABLE_WINDOW_RETURN",
    "returnWindowName",
    "awaitingReturn",
  ]
  for key in debugKeys {
    DebugGUI.fields.Push( { Key: key, Label: key ":" } )
  }

  CreateGUIElements( DebugGUI )
  DebugGUI.reloadGuiBound := ReloadGUI.Bind( DebugGUI )
  DebugGUI.gui.OnEvent( "Size", OnResizeKeepTextRightAligned.Bind( DebugGUI ) )

  return DebugGUI
}
