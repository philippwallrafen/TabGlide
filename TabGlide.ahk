/*
 * This file is part of TabGlide (https://github.com/e4zyphil/TabGlide).
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
global ALLOWED_PROGRAMS := BuildLowercaseMap( [
  "brave.exe",
  "chrome.exe",
  "chromium.exe",
  "explorer.exe",
  "firefox.exe",
  "msedge.exe",
  "opera.exe",
  "opera_gx.exe",
  "WindowsTerminal.exe",
] )
global TOP_REGION_PIXEL_LIMIT := 50     ; Fine tune - How forgiving the Y coordinate is
global ENABLE_FOCUS_RETURN := true      ; Controls whether focus returns to the previous window (true) or remains on the new one (false)
global RETURN_AFTER_MS := 700           ; Fine tune - Return to the previous active window after _ in ms
global DEBUG := false                   ; Debug mode
global DEBUG_GUI_BIND := "$F12"         ; Debug bind

;; GENERAL & START-UP
;; =============================================
#Requires AutoHotkey v2
#SingleInstance Force
A_MaxHotkeysPerInterval := 200
InstallMouseHook( true, true )
CoordMode( "Mouse", "Screen" )
global awaitingRefocus := false
global refocusWindowId := -1

; Builds ALLOWED_PROGRAMS as a map with lowercase keys and true values
BuildLowercaseMap( array ) {
  lowercaseMap := Map()
  for program in array
    lowercaseMap[ StrLower( program ) ] := true
  return lowercaseMap
}

; If using compiled version create TabGlide_config.ini and read content from there
if ( A_IsCompiled ) {
  global configFile := A_ScriptDir "\TabGlide_config.ini"
  Log( "info", "TabGlide.exe started" )
  if ( !FileExist( configFile ) ) {
    CreateDefaultConfig( configFile )
  }
  LoadConfig( configFile )
  Log( "debug", "TabGlide_config.ini loaded" )
  Log( "debug", "ALLOWED_PROGRAMS: " FormatMapToString( ALLOWED_PROGRAMS ) )
  Log( "debug", "TOP_REGION_PIXEL_LIMIT: " TOP_REGION_PIXEL_LIMIT )
  Log( "debug", "ENABLE_FOCUS_RETURN: " ( ENABLE_FOCUS_RETURN ? "true" : "false" ) )
  Log( "debug", "RETURN_AFTER_MS: " RETURN_AFTER_MS )
  Log( "debug", "DEBUG: " ( DEBUG ? "true" : "false" ) )
  Log( "debug", "DEBUG_GUI_BIND: " DEBUG_GUI_BIND )
} else {
  Log( "info", "TabGlide.ahk started" )
}

;; HELPER FUNCTIONS: Config & Logging
;; =============================================
CreateDefaultConfig( configFile ) {
  local defaultConfig := "
    (
    [CONFIG]
    ; Add all suitable programs here :) I tried sorting them alphabetically
    ALLOWED_PROGRAMS = brave.exe,chrome.exe,chromium.exe,explorer.exe,firefox.exe,msedge.exe,opera.exe,opera_gx.exe,WindowsTerminal.exe

    ; Fine tune - How forgiving the Y coordinate is
    TOP_REGION_PIXEL_LIMIT = 50
    ; Controls whether focus returns to the previous window (true) or remains on the new one (false)
    ENABLE_FOCUS_RETURN = true
    ; Fine tune - Return to the previous active window after _ in ms
    RETURN_AFTER_MS = 700
    ; Debug mode
    DEBUG = false
    ; Debug bind
    DEBUG_GUI_BIND = $F12
    )"

  FileAppend( "; " configFile "`n`n" defaultConfig, configFile )
  Log( "info", "No TabGlide_config.ini found. Config created at: " configFile )
  Run( configFile )
}
LoadConfig( configFile ) {
  global ALLOWED_PROGRAMS, TOP_REGION_PIXEL_LIMIT, ENABLE_FOCUS_RETURN, RETURN_AFTER_MS, DEBUG, DEBUG_GUI_BIND
  local allowedProgramsFromIni := IniRead( configFile, "CONFIG", "ALLOWED_PROGRAMS", "" )
  if ( allowedProgramsFromIni ) {
    local programList := StrSplit( allowedProgramsFromIni, "," )
    for index, program in programList
      programList[ index ] := Trim( program )
    ALLOWED_PROGRAMS := BuildLowercaseMap( programList )
  }
  ENABLE_FOCUS_RETURN := StrLower( IniRead( configFile, "CONFIG", "ENABLE_FOCUS_RETURN", ENABLE_FOCUS_RETURN ) ) = "true"
  RETURN_AFTER_MS := IniRead( configFile, "CONFIG", "RETURN_AFTER_MS", RETURN_AFTER_MS ) + 0                         ; Ensure numeric conversion
  TOP_REGION_PIXEL_LIMIT := IniRead( configFile, "CONFIG", "TOP_REGION_PIXEL_LIMIT", TOP_REGION_PIXEL_LIMIT ) + 0    ; Ensure numeric conversion
  DEBUG := StrLower( IniRead( configFile, "CONFIG", "DEBUG", DEBUG ) ) = "true"
  DEBUG_GUI_BIND := IniRead( configFile, "CONFIG", "DEBUG_GUI_BIND", DEBUG_GUI_BIND )
}
FormatMapToString( dataMap ) {
  local formattedOutput := ""
  for key, value in dataMap {
    formattedOutput .= key ": " value "`n"
  }
  return formattedOutput
}
Log( level, message ) {
  if ( !DEBUG ) {
    return
  }
  formattedTime := "[" FormatTime( A_Now, "yyyy-MM-dd HH:mm:ss" ) "] "
  formattedLevel := "[" StrUpper( level ) "] "
  FileAppend( formattedTime formattedLevel message "`n", A_ScriptDir "\TabGlide.log" )
}

;; HELPER FUNCTIONS: Main in order of execution
;; =============================================
GetProcessName( windowId ) {
  if ( windowId = -1 ) {
    Log( "debug", "GetProcessName(): refocusWindowId not set yet" )
    return "<invalid>"
  } else if ( !windowId || !WinExist( "ahk_id " windowId ) ) {
    Log( "info", "GetProcessName(): Window to get processName not found '" windowId "' - returning '<invalid>'." )
    return "<invalid>"
  }
  return WinGetProcessName( windowId )
}
GetRelativeMouseY() {
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  ; Retrieve monitorHandle per Dll for the current mouse position (MONITOR_DEFAULTTONEAREST = 2)
  local monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" )
  local monitorInfo := Buffer( 40, 0 )    ; Allocate a buffer for the MONITORINFO structure (40 bytes)
  NumPut( "UInt", 40, monitorInfo, 0 )    ; Set the structure size (dwSize = 40)
  ; Retrieve monitorInfo per Dll
  if ( !DllCall( "GetMonitorInfo", "Ptr", monitorHandle, "Ptr", monitorInfo ) ) {
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
GetFocusedWindowId() {
  local id := WinGetID( "A" )
  if ( !id ) {
    Log( "warning", "GetFocusedWindowId(): WinGetID returned '" id "' – returning 0." )
    return 0
  }
  return id
}
TrackScrollActivity() {
  ;;; global RETURN_AFTER_MS
  SetTimer( AfterTimerEnds, -RETURN_AFTER_MS )
}
AfterTimerEnds() {
  global awaitingRefocus, refocusWindowId
  local refocusWindowName := GetProcessName( refocusWindowId )
  if ( awaitingRefocus && refocusWindowName != "explorer.exe" ) {
    FocusWindow( refocusWindowId )
  }
  awaitingRefocus := false
}
FocusWindow( windowId ) {
  if ( !windowId || !WinExist( "ahk_id " windowId ) ) {
    Log( "warning", "FocusWindow(): Window to focus not found – ID: " windowId )
    return false
  }
  WinActivate( "ahk_id " windowId )
  return true
}

;; MAIN
;; =============================================
~$WheelUp::
~$WheelDown::
{
  ;;; global TOP_REGION_PIXEL_LIMIT, ENABLE_FOCUS_RETURN
  MouseGetPos(, , &mouseWindowId )
  local processName := GetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) )
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
    global refocusWindowId, awaitingRefocus
    if ( isUnfocusedWindow && !awaitingRefocus ) {
      refocusWindowId := focusedWindowId
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
;;; TODO: Implement if (paramStr == "ImmersiveColorSet") {InitializeDebugGUI}
; OnSystemThemeChanged( DebugGUI, wParam, lParam, msg, hwnd ) {
;   Log( "debug", "WM_SETTINGCHANGE received: wParam=" wParam ", lParam=" lParam ", msg=" msg ", hwnd=" hwnd )
;   if ( !lParam ) {
;     Log( "warning", "lParam is null." )
;     return 0
;   }
;   local paramStr := StrGet( lParam, "UTF-16" )
;   Log( "debug", "lParam string: " paramStr )
;   return 0
; }
IsWindowsDarkMode() {
  try {
    value := RegRead( "HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme" )
    return value = 0  ; 0 = dark mode, 1 = light mode
  } catch {
    Log( "warning", "IsWindowsDarkMode(): Could not read registry – defaulting to light mode" )
    return false
  }
}
SetLightMode( DebugGUI ) {
  DebugGUI.gui.BackColor := "ffffff"
  DebugGUI.gui.SetFont( "s14 q5 c1C1C1C", "Segoe UI" )
}
SetDarkMode( DebugGUI ) {
  DebugGUI.gui.BackColor := "1C1C1C"
  DebugGUI.gui.SetFont( "s14 q5 cffffff", "Segoe UI" )
  DarkModeTitleBar( DebugGUI.gui.Hwnd )
}
DarkModeTitleBar( Hwnd ) {
  local osVersion := StrSplit( A_OSVersion, "." ), major := osVersion[ 1 ] + 0, build := osVersion[ 3 ] + 0
  static DWMWA_USE_IMMERSIVE_DARK_MODE := 20
  ; Set dark title bar only when Windows version supports it
  if ( major >= 10 && build >= 17763 ) {
    DllCall( "dwmapi\DwmSetWindowAttribute", "ptr", Hwnd, "int", DWMWA_USE_IMMERSIVE_DARK_MODE, "int*", 1, "int", 4 )
  }
}
OnResizeKeepValuesRightAligned( DebugGUI, GuiObj, MinMax, Width, Height ) {
  if ( MinMax = -1 || MinMax = 1 ) {
    return    ; Ignore minimize/maximize
  }
  local guiFields := DebugGUI.guiFields, layout := DebugGUI.layout, fields := DebugGUI.fields
  for field in fields {
    guiFields[ field.Key ].Move( Width - layout.widthValue - 20, layout.startY + layout.lineSpacing * ( A_Index - 1 ) )
  }
  SetTimer( DebugGUI.reloadGui, -500 )
}
UpdateDebugGUI( DebugGUI ) {
  ;;; global ENABLE_FOCUS_RETURN, awaitingRefocus
  MouseGetPos( &mouseX, &mouseY, &mouseWindowId )
  local guiFields := DebugGUI.guiFields, processName := GetProcessName( mouseWindowId ), processInMap := ALLOWED_PROGRAMS.Has( StrLower( processName ) ), focusedWindowId := GetFocusedWindowId(), isUnfocusedWindow := mouseWindowId != focusedWindowId, monitorHandle := DllCall( "MonitorFromPoint", "int64", ( mouseX & 0xFFFFFFFF ) | ( mouseY << 32 ), "uint", 2, "ptr" ), relativeToMonitorY := GetRelativeMouseY(), refocusWindowName := GetProcessName( refocusWindowId )

  guiFields[ "ahkVersion" ].Text := A_AhkVersion
  guiFields[ "isAdmin" ].Text := A_IsAdmin ? "true" : "false"
  guiFields[ "mouseX" ].Text := mouseX
  guiFields[ "mouseY" ].Text := mouseY
  guiFields[ "monitorHandle" ].Text := monitorHandle
  guiFields[ "relativeToMonitorY" ].Text := relativeToMonitorY
  guiFields[ "processName" ].Text := processName
  guiFields[ "processInMap" ].Text := processInMap ? "true" : "false"
  guiFields[ "isUnfocusedWindow" ].Text := isUnfocusedWindow ? "true" : "false"
  guiFields[ "ENABLE_FOCUS_RETURN" ].Text := ENABLE_FOCUS_RETURN ? "true" : "false"
  guiFields[ "refocusWindowName" ].Text := refocusWindowName
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
  local gui := DebugGUI.gui, fields := DebugGUI.fields, layout := DebugGUI.layout, guiFields := DebugGUI.guiFields, xPos := layout.startX + layout.widthLabel + 10, wValue := layout.widthValue
  for field in fields {
    yPos := layout.startY + layout.lineSpacing * ( A_Index - 1 )
    ; Left side labels
    gui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans", layout.startX, yPos, layout.widthLabel ), field.Label )
    ; Right side values
    if ( field.Key = "processName" ) {
      guiFields[ field.Key ] := gui.Add( "Edit", Format( "x{} y{} w{} +ReadOnly -E0x200 Right", xPos, yPos, wValue ) )
      guiFields[ field.Key ].Opt( "+Background" gui.BackColor )
    } else {
      guiFields[ field.Key ] := gui.Add( "Text", Format( "x{} y{} w{} BackgroundTrans Right", xPos, yPos, wValue ) )
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
  local windowWidth := 400

  DebugGUI.gui.Title := "TabGlide"
  DebugGUI.gui.OnEvent( "Size", OnResizeKeepValuesRightAligned.Bind( DebugGUI ) )
  if ( IsWindowsDarkMode() = true ) {
    SetDarkMode( DebugGUI )
  } else {
    SetLightMode( DebugGUI )
  }
  ;;; TODO: Implement if (paramStr == "ImmersiveColorSet") {InitializeDebugGUI}
  ; OnMessage( 0x001A, OnSystemThemeChanged.Bind( DebugGUI ) )

  debugKeys := [
    "ahkVersion",
    "isAdmin",
    "mouseX",
    "mouseY",
    "monitorHandle",
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
  DebugGUI.reloadGui := ReloadGUI.Bind( DebugGUI )

  DebugGUI.gui.Show( "Hide w" windowWidth )
  DebugGUI.gui.Hide()
  return DebugGUI
}

;; DEBUG-GUI: Bind
;; =============================================
if ( DEBUG ) {
  Hotkey( DEBUG_GUI_BIND, ( * ) => DebugBind(), "On" )
}
DebugBind() {
  static DebugGUI := InitializeDebugGUI()
  UpdateDebugGUI( DebugGUI )
}
