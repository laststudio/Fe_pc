unit WL3SplashScreen;

{$mode Delphi}

interface

procedure ShowSplashScreen(ABitmapFilename : String; AWindowTitleCaption : String = 'WL3SplashScreen'; AWindowClassName : String = 'WL3SplashScreenClass');
procedure HideSplashScreen;

implementation
uses JwaWinUser, Windows, Messages, Classes, SysUtils;

// Do NOT use resources, use WinLicense XBundler splash screen files, it's much faster
{.$R SplashScreen.rc}

type
     TSplashScreen = class
     private
        FBitmapFilename : String;
        FWndClassName : String;
        FWndTitleCaption : String;

        FWndHandle: HWND;
        FEndLoopNow : Boolean;

        function MakeWindow : String;
        procedure MessageLoop;
        procedure ReleaseResources;
     public
        constructor Create(ABitmapFilename, AWindowTitleCaption, AWindowClassName : String);
     end;


var SplashScreen : TSplashScreen = nil;

procedure ShowSplashScreen(ABitmapFilename, AWindowTitleCaption, AWindowClassName : String);
begin
   SplashScreen := TSplashScreen.Create(ABitmapFilename, AWindowTitleCaption, AWindowClassName);

   TThread.ExecuteInThread(SplashScreen.MessageLoop);
end;

procedure HideSplashScreen;
begin
   if not Assigned(SplashScreen) then exit;
   SplashScreen.ReleaseResources;
end;


function WindowProc(hWnd, Msg : Longint; wParam : WPARAM; lParam: LPARAM) : Longint; stdcall;
begin
   case Msg of

       // https://devblogs.microsoft.com/oldnewthing/20191014-00/?p=102992
       WM_NCCREATE  : SplashScreen.FWndHandle := hwnd;

       // close on mouse lcick
       WM_LBUTTONUP,

       // Alt+F4 or PostMessage(hwnd, WM_CLOSE, ...) from user app
       WM_CLOSE     : HideSplashScreen;
   end;

   Result := DefWindowProc(hWnd, Msg, wParam, lParam);
end;

constructor TSplashScreen.Create(ABitmapFilename, AWindowTitleCaption, AWindowClassName : String);
begin
   inherited Create;

   FBitmapFilename := ABitmapFilename;
   FWndTitleCaption := AWindowTitleCaption;
   FWndClassName := AWindowClassName;

   FWndHandle := 0;
   FEndLoopNow := False;
end;

procedure TSplashScreen.MessageLoop;
var msg : TMSG;
    error : String;
begin
   error := MakeWindow;
   if not error.IsEmpty then begin
   {$IFDEF Debug}
      MessageBox(0, PChar(error + ':' + sLineBreak + SysErrorMessage(GetLastError)), 'MakeWindow', MB_OK or MB_ICONERROR);
   {$ENDIF}
      exit;
   end;

   // main message loop
   while GetMessage(msg, 0, 0, 0) do begin
         if FEndLoopNow then
            break;

         TranslateMessage(msg);
         DispatchMessage(msg);
   end;
end;

procedure TSplashScreen.ReleaseResources;
begin
   FEndLoopNow := True;
   ShowWindow(SplashScreen.FWndHandle, SW_HIDE);
   DestroyWindow(SplashScreen.FWndHandle);
   UnregisterClassA(PChar(FWndClassName), HInstance);
   Free;
end;


const HGDI_ERROR = {$IFDEF WIN64}$FFFFFFFF;{$ENDIF} {$IFDEF WIN32}$FFFF;{$ENDIF}

function TSplashScreen.MakeWindow : String;
var
    aWndClass  : TWndClass;
    aHwnd : HWND;

    aPos : TPoint;
    aSize : TSize;
    aBlendFunction : TBlendFunction;

    dpi, bmpWidth, bmpHeight : Integer;

    filename : String;
    aBmp : HBitmap;
    aBitmap : Bitmap;
    aHdcScreen, aHdc : HDC;
    aBmpOld : HBITMAP;
    workArea : TRECT;

    procedure FreeResources;
    begin
      SelectObject(aHdc, aBmpOld);
      DeleteDC(aHdc);
      DeleteObject(aBmp);
    end;

begin
   Result := '';

   // get the Screen DC
   aHdcScreen := GetDC(0);
   if aHdcScreen = 0 then exit('No screen DC.');

   // load the 32bit image to show = BMP with alpha channel
   filename := FBitmapFilename;
   aBmp := LoadImageA(0, PChar(filename), IMAGE_BITMAP, 0, 0, LR_LOADFROMFILE);
   if aBmp = 0 then exit('Could not load BMP '+filename);

   // get a compatible DC
   aHdc := CreateCompatibleDC(aHdcScreen);
   ReleaseDC(0, aHdcScreen);
   if aHdc = 0 then begin
      DeleteObject(aBmp);
      exit('No compaitble DC.');
   end;

   // select the loaded bitmap
   aBmpOld := SelectObject(aHdc, aBmp);
   if (aBmpOld = 0) or (aBmpOld = HGDI_ERROR) then begin
      DeleteDC(aHdc);
      DeleteObject(aBmp);
      exit('Could not select the bmp handle.');
   end;

   // get the bitmap infos
   if GetObject(aBmp, sizeof(aBitmap), @aBitmap) = 0 then begin
      FreeResources; exit('Could not get bitmap infos.');
   end;

   bmpWidth := aBitmap.bmWidth;
   bmpHeight := aBitmap.bmHeight;

   // init the windows class
   FillChar(aWndClass, sizeof(aWndClass), 0);
   aWndClass.hInstance := HInstance;
   with aWndClass do begin
       lpszClassName := PChar(FWndClassName);
       Style         := CS_PARENTDC or CS_BYTEALIGNCLIENT;
       hIcon         := 0;
       lpfnWndProc   := @WindowProc;
       hbrBackground := COLOR_BTNFACE+1;
       hCursor       := LoadCursor(0,IDC_ARROW);
   end;

   // register the window class
   if Windows.RegisterClass(aWndClass) = 0 then begin
      FreeResources; exit('Could not register window class.');
   end;

   workArea := Rect(0, 0, 0, 0);
   if not SystemParametersInfo(SPI_GETWORKAREA, 0, @workArea, 0) then begin
      workArea.Width := GetSystemMetrics(SM_CXSCREEN);
      workArea.Height := GetSystemMetrics(SM_CYSCREEN);
   end;

   // create a centered and fully transparent window
   aHwnd := CreateWindowEx(
      WS_EX_LAYERED,   // makes it transparent
      aWndClass.lpszClassName,
      PChar(FWndTitleCaption),
      WS_SYSMENU or WS_VISIBLE,
      workArea.Left + ((workArea.Width - bmpWidth) div 2),
      workArea.Top + ((workArea.Height - bmpHeight) div 2),
      bmpWidth, bmpHeight, 0,0,
      hInstance,
      nil
   );
   if aHWnd = 0 then begin
      FreeResources; exit('Could not create the splash screen window.');
   end;

   // https://devblogs.microsoft.com/oldnewthing/20191014-00/?p=102992
   if FWndHandle = 0 then
      FWndHandle := aHwnd;

   // set the semi transparent image as window layer
   aPos := Point(0, 0);
   aSize.cx := bmpWidth;
   aSize.cy := bmpHeight;

   aBlendFunction.BlendOp := AC_SRC_OVER;
   aBlendFunction.BlendFlags := 0 ;
   aBlendFunction.SourceConstantAlpha := 255;
   aBlendFunction.AlphaFormat := AC_SRC_ALPHA ;

   if not UpdateLayeredWindow(FWndHandle, 0, nil, @aSize, aHdc, @aPos, 0, @aBlendFunction, ULW_ALPHA) then
      Result := 'Failed to update the transparent window layer with bitmap.';

   FreeResources;
end;


end.

