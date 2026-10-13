Add-Type -AssemblyName System.Drawing
$root=Split-Path $PSScriptRoot -Parent
$bitmap=New-Object Drawing.Bitmap 256,256
$g=[Drawing.Graphics]::FromImage($bitmap)
$g.SmoothingMode=[Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([Drawing.Color]::Transparent)
$background=New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(49,105,84))
$g.FillEllipse($background,8,8,240,240)
$pen=New-Object Drawing.Pen ([Drawing.Color]::FromArgb(246,248,239)),14
$pen.StartCap=[Drawing.Drawing2D.LineCap]::Round
$pen.EndCap=[Drawing.Drawing2D.LineCap]::Round
$g.DrawLine($pen,72,112,72,144)
$g.DrawLine($pen,100,84,100,172)
$g.DrawLine($pen,128,65,128,191)
$g.DrawLine($pen,156,88,156,168)
$g.DrawLine($pen,184,112,184,144)
$bitmap.Save((Join-Path $root 'assets/whispering.png'),[Drawing.Imaging.ImageFormat]::Png)
$icon=[Drawing.Icon]::FromHandle($bitmap.GetHicon())
$stream=[IO.File]::Create((Join-Path $root 'assets/whispering.ico'))
$icon.Save($stream)
$stream.Dispose(); $icon.Dispose(); $g.Dispose(); $pen.Dispose(); $background.Dispose(); $bitmap.Dispose()
