program WL3SplashScreenDebugApp;

function SecureEngineInitialize: Boolean; stdcall; external 'WL3SplashScreenPlugin.dll' name 'SecureEngineInitialize';
function SecureEngineFinalize:   Boolean; stdcall; external 'WL3SplashScreenPlugin.dll' name 'SecureEngineFinalize';

{$R *.res}

begin
   WriteLn('Press [enter] to hide the splas screen.');
   SecureEngineInitialize;
   Readln;
   SecureEngineFinalize;
   WriteLn('Press [enter] to exit the demo app.');
   Readln;
end.

