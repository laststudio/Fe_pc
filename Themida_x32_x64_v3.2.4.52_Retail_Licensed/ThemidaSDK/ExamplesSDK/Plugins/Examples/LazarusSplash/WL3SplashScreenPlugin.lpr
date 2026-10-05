library WL3SplashScreenPlugin;

{$mode delphi}{$H+}

uses Windows, SysUtils, WL3SplashScreen;

function GetFilenameSuffix : String;
var aDC : HDC;
    dpi : Integer;
begin
   Result := 'Failed';

   // get the Screen DC of primary monitor
   aDC := GetDC(0);
   if aDC = 0 then exit;

   // get DPI of primary monitor
   dpi := GetDeviceCaps(aDC, LOGPIXELSX);
   ReleaseDC(0, aDC);

   case dpi of
         96 : Result := '';       // 100% display scaling
        120 : Result := '@1.25x'; // 125%
        144 : Result := '@1.5x';  // 150%
        168 : Result := '@1.5x';  // 175%
        192 : Result := '@2x';    // 200%
        216 : Result := '@2x';    // 225%
        240 : Result := '@2x';    // 250%
        288 : Result := '@3x';    // 300%
        336 : Result := '@3x';    // 350%
   else
              Result := '@1.5x';
   end;
end;

function GetBitmapFilename : String;
var suffix : String;
begin
   Result := '';
   suffix := GetFilenameSuffix;
   if suffix = 'Failed' then exit;

   Result := Format('SplashScreen%s.bmp',[suffix]);
end;

function SecureEngineInitialize: Boolean; stdcall; export;
begin
   Result := True;
   ShowSplashScreen(GetBitmapFilename, 'MySplashScreen', 'MySplashClass');
   // use PostMessage(FindWindow('MySplashClass', 'MySpalshScreen'), WM_CLOSE, 0, 0); in the user app to close the splash screen
end;

function SecureEngineFinalize: Boolean; stdcall; export;
begin
   Result := True;
   HideSplashScreen;
end;

exports
   SecureEngineInitialize,
   SecureEngineFinalize;

begin
end.

