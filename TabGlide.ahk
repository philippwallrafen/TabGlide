/*
 * This file is part of TabGlide (https://github.com/e4zyphil/tabglide).
 * Copyright (c) 2025 Philipp Wallrafen
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * See the file LICENSE for the full license text.
*/

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
global ENABLE_FOCUS_RETURN := true      ; Controls whether focus returns to the previous window (true) or remains on the new one (false)
global RETURN_AFTER_MS := 500           ; Fine tune - Return to the previous active window after _ in ms
global TOP_REGION_PIXEL_LIMIT := 50     ; Fine tune - How forgiving the Y coordinate is
global DEBUG := true                   ; Debug mode
global DEBUG_GUI_BIND := "$F11"         ; Debug bind

;; GENERAL & START-UP
;; =============================================
#Requires AutoHotkey v2
#SingleInstance Force
A_MaxHotkeysPerInterval := 200
InstallMouseHook( true, true )
CoordMode( "Mouse", "Screen" )
global awaitingRefocus := false
global refocusWindow := ""

; Convert ALLOWED_PROGRAMS to lowercase
lowercasedMap := Map()
for program, value in ALLOWED_PROGRAMS {
  lowercasedMap[ StrLower( program ) ] := value
}
ALLOWED_PROGRAMS := lowercasedMap

;; HELPER FUNCTIONS
;; =============================================
GetFocusedWindowId() {
  try {
    return WinGetID( "A" )
  } catch {
    Log( "warning", "No active window found." )
    return 0
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
  global awaitingRefocus, refocusWindow
  if ( awaitingRefocus ) {
    FocusWindow( refocusWindow )
    awaitingRefocus := false
  }
}
FocusWindow( windowId ) {
  try {
    WinActivate( windowId )
    return true
  } catch {
    Log( "warning", "Window to activate not found." )
    return false
  }
}
Log( level, message ) {
  if ( !DEBUG ) {
    return
  }
  formattedTime := "[" FormatTime( A_Now, "yyyy-MM-dd HH:mm:ss" ) "] "
  formattedLevel := "[" StrUpper( level ) "] "
  FileAppend( formattedTime formattedLevel message "`n", A_ScriptDir "\tabglide.log" )
}

;; MAIN
;; =============================================
~$WheelDown::
~$WheelUp::
{
  ;;; global TOP_REGION_PIXEL_LIMIT, ENABLE_FOCUS_RETURN
  MouseGetPos(, , &mouseWindowId )
  local processName := WinGetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) )
  if ( !processInMap ) {
    return
  }
  local relativeToMonitorY := GetRelativeMouseY()
  if ( relativeToMonitorY > TOP_REGION_PIXEL_LIMIT ) {
    return
  }
  local focusedWindowId := GetFocusedWindowId(), isUnfocusedWindow := mouseWindowId != focusedWindowId
  ; (optional) Return to original window
  if ( ENABLE_FOCUS_RETURN ) {
    TrackScrollActivity()
    global refocusWindow, awaitingRefocus
    if ( isUnfocusedWindow && !awaitingRefocus ) {
      refocusWindow := focusedWindowId
      awaitingRefocus := true
    }
  }
  if ( isUnfocusedWindow ) {
    FocusWindow( mouseWindowId )
  }
  if ( A_ThisHotkey = "~$WheelDown" ) {
    Send( "^{Tab}" )
  } else {
    Send( "^+{Tab}" )
  }
}

;; DEBUG-MODE
;; =============================================
if ( !DEBUG ) {
  return
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
  ;;; global ENABLE_FOCUS_RETURN, awaitingRefocus
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  local guiFields := DebugGUI.guiFields, processName := WinGetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) ), focusedWindowId := GetFocusedWindowId(), isUnfocusedWindow := mouseWindowId != focusedWindowId, monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" ), relativeToMonitorY := GetRelativeMouseY()

  guiFields[ "ahkVersion" ].Text := A_AhkVersion
  guiFields[ "isAdmin" ].Text := A_IsAdmin ? "true" : "false"
  guiFields[ "mouseX" ].Text := mouseX
  guiFields[ "monitorHandle" ].Text := monitorHandle
  guiFields[ "mouseY" ].Text := mouseY
  guiFields[ "relativeToMonitorY" ].Text := relativeToMonitorY
  guiFields[ "processName" ].Text := processName
  guiFields[ "processInMap" ].Text := processInMap ? "true" : "false"
  guiFields[ "isUnfocusedWindow" ].Text := isUnfocusedWindow ? "true" : "false"
  guiFields[ "ENABLE_FOCUS_RETURN" ].Text := ENABLE_FOCUS_RETURN ? "true" : "false"
  guiFields[ "refocusWindowName" ].Text := WinGetProcessName( mouseWindowId )
  guiFields[ "awaitingRefocus" ].Text := awaitingRefocus ? "true" : "false"

  guiFields[ "processName" ].SetFont( "Bold" )
  DebugGUI.gui.GetPos(, , &guiWidth )
  DebugGUI.gui.Show( Format( "x{} y{}", mouseX - ( guiWidth / 2 ), mouseY + 10 ) )
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
    "isUnfocusedWindow",
    "ENABLE_FOCUS_RETURN",
    "refocusWindowName",
    "awaitingRefocus",
  ]
  for key in debugKeys {
    DebugGUI.fields.Push( { Key: key, Label: key ":" } )
  }
  CreateGUIElements( DebugGUI )
  DebugGUI.reloadGuiBound := ReloadGUI.Bind( DebugGUI )
  DebugGUI.gui.OnEvent( "Size", OnResizeKeepTextRightAligned.Bind( DebugGUI ) )
  DebugGUI.gui.Show( "Hide y10000 w380" )
  DebugGUI.gui.Hide
  return DebugGUI
}
;; DEBUG-GUI: Bind
;; =============================================
if ( DEBUG ) {
  DebugGUI := InitializeDebugGUI()
  Hotkey( DEBUG_GUI_BIND, ( * ) => DebugBind( DebugGUI ), "On" )
}
DebugBind( DebugGUI ) {
  UpdateDebugGUI( DebugGUI )
}
