;; Wheel Scroll Tabs for Google Chrome 

#IfWinActive ahk_class Chrome_WidgetWin_1 
 ~$WheelDown:: 
 ~$WheelUp:: 
    MouseGetPos,, yaxis 
    IfGreater,yaxis,50, Return 
    IfEqual,A_ThisHotkey,~$WheelDown, Send ^{PgDn} 
                                 Else Send ^{PgUp} 
Return 
#IfWinActive