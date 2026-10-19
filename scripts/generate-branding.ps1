$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$out = Join-Path $root 'assets/branding'
[IO.Directory]::CreateDirectory($out) | Out-Null
$svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect x="2" y="2" width="60" height="60" rx="14" fill="#242a31"/><path d="M26 13h12v21h11L32 51 15 34h11z" fill="#ef8b43"/><path d="M18 55h28" stroke="#ffffff" stroke-width="4" stroke-linecap="round"/></svg>'
[IO.File]::WriteAllText((Join-Path $out 'rusty.svg'), $svg)
$sizes = @(16,20,24,32,40,48,64,128,256)
$frames = [Collections.Generic.List[byte[]]]::new()
foreach ($size in $sizes) {
    $bitmap = [Drawing.Bitmap]::new($size * 4, $size * 4)
    $g = [Drawing.Graphics]::FromImage($bitmap)
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.ScaleTransform(($size / 16.0), ($size / 16.0))
    $path = [Drawing.Drawing2D.GraphicsPath]::new()
    foreach ($arc in @(@(2,2,180),@(34,2,270),@(34,34,0),@(2,34,90))) {
        $path.AddArc([float]$arc[0],[float]$arc[1],28,28,[float]$arc[2],90)
    }
    $path.CloseFigure()
    $dark = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#242a31'))
    $orange = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#ef8b43'))
    $pen = [Drawing.Pen]::new([Drawing.Color]::White,4)
    $pen.StartCap = $pen.EndCap = [Drawing.Drawing2D.LineCap]::Round
    try {
        $g.FillPath($dark,$path)
        $points = [Drawing.PointF[]]@([Drawing.PointF]::new(26,13),[Drawing.PointF]::new(38,13),[Drawing.PointF]::new(38,34),[Drawing.PointF]::new(49,34),[Drawing.PointF]::new(32,51),[Drawing.PointF]::new(15,34),[Drawing.PointF]::new(26,34))
        $g.FillPolygon($orange,$points)
        $g.DrawLine($pen,18,55,46,55)
        $small = [Drawing.Bitmap]::new($size,$size)
        $sg = [Drawing.Graphics]::FromImage($small)
        $sg.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $sg.DrawImage($bitmap,0,0,$size,$size)
        $stream = [IO.MemoryStream]::new()
        try {
            $small.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
            $bytes = $stream.ToArray(); $frames.Add($bytes)
            [IO.File]::WriteAllBytes((Join-Path $out "rusty-$size.png"),$bytes)
            if ($size -in @(16,32,48,128)) { [IO.File]::WriteAllBytes((Join-Path $root "extensions/chrome/icons/rusty-$size.png"),$bytes) }
        } finally { $stream.Dispose(); $sg.Dispose(); $small.Dispose() }
    } finally { $pen.Dispose(); $orange.Dispose(); $dark.Dispose(); $path.Dispose(); $g.Dispose(); $bitmap.Dispose() }
}
$writer = [IO.BinaryWriter]::new([IO.File]::Create((Join-Path $out 'rusty.ico')))
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    for ($i=0; $i -lt $sizes.Count; $i++) {
        $dimension = if ($sizes[$i] -eq 256) { 0 } else { $sizes[$i] }
        $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
        $offset += $frames[$i].Length
    }
    foreach ($frame in $frames) { $writer.Write($frame) }
} finally { $writer.Dispose() }
Write-Output 'Generated original Rusty branding: SVG, nine PNG/ICO sizes and Chrome icons.'
