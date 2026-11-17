Add-Type -AssemblyName System.Drawing

$size = 1024
$out = Join-Path $PSScriptRoot 'app-icon.png'
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$g.Clear([System.Drawing.Color]::Transparent)

$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$r = 220
$path.AddArc(0, 0, $r, $r, 180, 90)
$path.AddArc($size - $r, 0, $r, $r, 270, 90)
$path.AddArc($size - $r, $size - $r, $r, $r, 0, 90)
$path.AddArc(0, $size - $r, $r, $r, 90, 90)
$path.CloseFigure()

$rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  $rect,
  [System.Drawing.ColorTranslator]::FromHtml('#5b6ff0'),
  [System.Drawing.ColorTranslator]::FromHtml('#3a4bd0'),
  45
)
$g.FillPath($brush, $path)

$font = New-Object System.Drawing.Font('Segoe UI', 480, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = [System.Drawing.StringAlignment]::Center
$fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
$white = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$textRect = New-Object System.Drawing.RectangleF(0, -18, $size, $size)
$g.DrawString('T', $font, $white, $textRect, $fmt)

$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Output $out
