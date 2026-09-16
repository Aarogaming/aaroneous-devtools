# dev/tools/bake_icons.ps1
# Native Windows In-House Multi-Resolution Icon & Asset Baker
# Generates .ico (256, 128, 64, 48, 32, 16) and master 512x512 .png files.

Add-Type -AssemblyName System.Drawing

function New-SquirclePath {
    param([float]$x, [float]$y, [float]$w, [float]$h, [float]$r)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function Save-MultiResIcon {
    param(
        [System.Drawing.Bitmap]$MasterBitmap,
        [string]$OutputIcoPath,
        [string]$OutputPngPath
    )

    $MasterBitmap.Save($OutputPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host "  [+] Master PNG: $OutputPngPath"

    $sizes = @(256, 128, 64, 48, 32, 16)
    $pngBytesList = New-Object System.Collections.Generic.List[byte[]]

    foreach ($s in $sizes) {
        $resized = New-Object System.Drawing.Bitmap $s, $s
        $g = [System.Drawing.Graphics]::FromImage($resized)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.DrawImage($MasterBitmap, 0, 0, $s, $s)
        $g.Dispose()

        $ms = New-Object System.IO.MemoryStream
        $resized.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $resized.Dispose()
        $pngBytesList.Add($ms.ToArray())
        $ms.Dispose()
    }

    # Write to temporary file first to handle Windows memory-mapped open locks cleanly
    $tmpIcoPath = $OutputIcoPath + ".tmp"
    $bakIcoPath = $OutputIcoPath + ".bak"

    $fs = [System.IO.FileStream]::new($tmpIcoPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::ReadWrite)
    $bw = New-Object System.IO.BinaryWriter $fs
    try {
        $bw.Write([uint16]0)
        $bw.Write([uint16]1)
        $bw.Write([uint16]$sizes.Count)

        $offset = 6 + (16 * $sizes.Count)

        for ($i = 0; $i -lt $sizes.Count; $i++) {
            $s = $sizes[$i]
            $bytes = $pngBytesList[$i]
            $w = if ($s -eq 256) { 0 } else { [byte]$s }
            $bw.Write([byte]$w)
            $bw.Write([byte]$w)
            $bw.Write([byte]0)
            $bw.Write([byte]0)
            $bw.Write([uint16]1)
            $bw.Write([uint16]32)
            $bw.Write([uint32]$bytes.Length)
            $bw.Write([uint32]$offset)
            $offset += $bytes.Length
        }

        for ($i = 0; $i -lt $sizes.Count; $i++) {
            $bw.Write($pngBytesList[$i])
        }
        $bw.Flush()
    }
    finally {
        $bw.Dispose()
        $fs.Dispose()
    }

    if (Test-Path $OutputIcoPath) {
        if (Test-Path $bakIcoPath) { Remove-Item $bakIcoPath -Force -ErrorAction SilentlyContinue }
        Move-Item -LiteralPath $OutputIcoPath -Destination $bakIcoPath -Force
        Move-Item -LiteralPath $tmpIcoPath -Destination $OutputIcoPath -Force
        Remove-Item -LiteralPath $bakIcoPath -Force -ErrorAction SilentlyContinue
    } else {
        Move-Item -LiteralPath $tmpIcoPath -Destination $OutputIcoPath -Force
    }

    Write-Host "  [+] Multi-Res ICO: $OutputIcoPath"
}

# ==============================================================================
# 1. BAKE AARONEOUS COMPUTE (SACRED HEX-LATTICE)
# ==============================================================================
Write-Host "Baking Aaroneous (Compute Sacred Hex-Lattice) Icon..."
$bmpCompute = New-Object System.Drawing.Bitmap 512, 512
$g = [System.Drawing.Graphics]::FromImage($bmpCompute)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Carbon Squircle
$bgPath = New-SquirclePath 20 20 472 472 108
$bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(0, 0),
    [System.Drawing.PointF]::new(512, 512),
    [System.Drawing.Color]::FromArgb(255, 24, 27, 35),
    [System.Drawing.Color]::FromArgb(255, 8, 9, 12)
)
$g.FillPath($bgBrush, $bgPath)
$borderPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 38, 45, 61)), 4
$g.DrawPath($borderPen, $bgPath)

# Blueprint Background Lattice (Subtle)
$gridPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(35, 0, 240, 255)), 2
$g.DrawEllipse($gridPen, 256 - 190, 256 - 190, 380, 380)
$g.DrawEllipse($gridPen, 256 - 145, 256 - 145, 290, 290)

# Sacred Equilateral Interlaced Triangles
$tri1 = @(
    [System.Drawing.PointF]::new(256, 100),
    [System.Drawing.PointF]::new(405, 358),
    [System.Drawing.PointF]::new(107, 358)
)
$tri2 = @(
    [System.Drawing.PointF]::new(256, 412),
    [System.Drawing.PointF]::new(107, 154),
    [System.Drawing.PointF]::new(405, 154)
)

$amberPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 255, 158, 68)), 5
$amberPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$cyanPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 5
$cyanPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

$g.DrawPolygon($amberPen, $tri1)
$g.DrawPolygon($cyanPen, $tri2)

# Peripheral Hex Lattice Connectors
$perimPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 190, 255)), 3.5
$nodes = @(
    [System.Drawing.PointF]::new(256, 100),
    [System.Drawing.PointF]::new(390, 177),
    [System.Drawing.PointF]::new(390, 335),
    [System.Drawing.PointF]::new(256, 412),
    [System.Drawing.PointF]::new(122, 335),
    [System.Drawing.PointF]::new(122, 177)
)
$g.DrawPolygon($perimPen, $nodes)

# Radial Convergence Channels
$g.DrawLine($amberPen, 256, 100, 256, 210)
$g.DrawLine($amberPen, 390, 177, 295, 232)
$g.DrawLine($cyanPen, 390, 335, 295, 280)
$g.DrawLine($cyanPen, 256, 412, 256, 302)
$g.DrawLine($cyanPen, 122, 335, 217, 280)
$g.DrawLine($amberPen, 122, 177, 217, 232)

# 6 Outer Satellite Hexagons
$satHexRadius = 24
foreach ($center in $nodes) {
    $satPts = @()
    for ($a = 0; $a -lt 360; $a += 60) {
        $rad = [Math]::PI * $a / 180.0
        $satPts += [System.Drawing.PointF]::new(
            $center.X + $satHexRadius * [Math]::Cos($rad),
            $center.Y + $satHexRadius * [Math]::Sin($rad)
        )
    }
    $darkBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 17, 20, 28))
    $g.FillPolygon($darkBrush, $satPts)
    if ($center.Y -lt 256) {
        $g.DrawPolygon($amberPen, $satPts)
    } else {
        $g.DrawPolygon($cyanPen, $satPts)
    }
    $beadBrush = if ($center.Y -lt 256) {
        New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 209, 102))
    } else {
        New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 0, 240, 255))
    }
    $g.FillEllipse($beadBrush, $center.X - 5, $center.Y - 5, 10, 10)
}

# Core Intermediate Hexagon
$innerHex = @()
for ($a = 30; $a -lt 390; $a += 60) {
    $rad = [Math]::PI * $a / 180.0
    $innerHex += [System.Drawing.PointF]::new(
        256 + 62 * [Math]::Cos($rad),
        256 + 62 * [Math]::Sin($rad)
    )
}
$g.FillPolygon((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 15, 18, 25))), $innerHex)
$g.DrawPolygon($amberPen, $innerHex)

# Thermodynamic Energy Singularity (Pulsing Amber Sun)
$plasmaBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(220, 220),
    [System.Drawing.PointF]::new(292, 292),
    [System.Drawing.Color]::FromArgb(255, 255, 220, 160),
    [System.Drawing.Color]::FromArgb(255, 216, 67, 21)
)
$g.FillEllipse($plasmaBrush, 256 - 32, 256 - 32, 64, 64)
$g.FillEllipse([System.Drawing.Brushes]::White, 256 - 14, 256 - 14, 28, 28)

$g.Dispose()

Save-MultiResIcon -MasterBitmap $bmpCompute -OutputIcoPath "D:\Aaroneous\assets\icons\aaroneous_core.ico" -OutputPngPath "D:\Aaroneous\assets\icons\aaroneous_core.png"
$bmpCompute.Dispose()

# ==============================================================================
# 2. BAKE ARC TRAJECTORY (SUPERSONIC HUD RUNTIME CONTROLLER)
# ==============================================================================
Write-Host "Baking Aaroneous Runtime Controller (ARC Trajectory) Icon..."
$bmpAfc = New-Object System.Drawing.Bitmap 512, 512
$g = [System.Drawing.Graphics]::FromImage($bmpAfc)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Cockpit Titanium Squircle
$bgPath = New-SquirclePath 20 20 472 472 108
$bgBrushAfc = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(0, 0),
    [System.Drawing.PointF]::new(512, 512),
    [System.Drawing.Color]::FromArgb(255, 21, 24, 33),
    [System.Drawing.Color]::FromArgb(255, 6, 7, 10)
)
$g.FillPath($bgBrushAfc, $bgPath)
$borderPenAfc = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 34, 40, 56)), 4
$g.DrawPath($borderPenAfc, $bgPath)

# Aerospace Compass HUD Outer Ring
$compassPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 4
$compassPen.DashPattern = @(4.0, 3.0, 8.0, 3.0)
$g.DrawEllipse($compassPen, 256 - 186, 256 - 186, 372, 372)
$darkCompassPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 27, 34, 48)), 3
$g.DrawEllipse($darkCompassPen, 256 - 186, 256 - 186, 372, 372)

# Cardinal Tick Marks
$tickPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 4
$tickPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$tickPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawLine($tickPen, 256, 58, 256, 82)
$g.DrawLine($tickPen, 430, 256, 454, 256)
$g.DrawLine($tickPen, 256, 430, 256, 454)
$g.DrawLine($tickPen, 58, 256, 82, 256)

# Orbiting Telemetry Satellite Beacons
$amberSatBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 160, 68))
$cyanSatBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 0, 240, 255))
$g.FillEllipse($amberSatBrush, 180 - 7, 85 - 7, 14, 14)
$g.FillEllipse($cyanSatBrush, 430 - 6, 200 - 6, 12, 12)
$g.FillEllipse($amberSatBrush, 340 - 8, 425 - 8, 16, 16)
$g.FillEllipse($cyanSatBrush, 85 - 6, 330 - 6, 12, 12)

# Gimbal Track Arc
$gimbalPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 37, 48, 68)), 2
$gimbalPen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dash
$g.DrawEllipse($gimbalPen, 256 - 118, 256 - 118, 236, 236)

# Curved Supersonic Ion Thrust Plume Wake
$wakePath = New-Object System.Drawing.Drawing2D.GraphicsPath
$wakePath.AddBezier(72, 440, 124, 374, 196, 304, 230, 282)

$wakePenOuter = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(70, 0, 166, 255)), 28
$wakePenOuter.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$wakePenOuter.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawPath($wakePenOuter, $wakePath)

$wakePenMid = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(180, 0, 240, 255)), 14
$wakePenMid.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$wakePenMid.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawPath($wakePenMid, $wakePath)

$wakePenCore = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 255, 255, 255)), 5
$wakePenCore.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$wakePenCore.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawPath($wakePenCore, $wakePath)

# Supersonic Interceptor Fighter Geometry (Heading 45° Northeast)
$jetPoints = @(
    [System.Drawing.PointF]::new(355, 157),
    [System.Drawing.PointF]::new(375, 275),
    [System.Drawing.PointF]::new(290, 240),
    [System.Drawing.PointF]::new(260, 310),
    [System.Drawing.PointF]::new(230, 282),
    [System.Drawing.PointF]::new(202, 252),
    [System.Drawing.PointF]::new(272, 222),
    [System.Drawing.PointF]::new(237, 137)
)
$jetDarkBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 14, 19, 28))
$g.FillPolygon($jetDarkBrush, $jetPoints)

$jetOutlinePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 7
$jetOutlinePen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$g.DrawPolygon($jetOutlinePen, $jetPoints)

# Cockpit Canopy Glass
$canopyPoints = @(
    [System.Drawing.PointF]::new(340, 172),
    [System.Drawing.PointF]::new(318, 214),
    [System.Drawing.PointF]::new(298, 194)
)
$canopyBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(230, 165, 243, 252))
$g.FillPolygon($canopyBrush, $canopyPoints)

# Afterburner Flare (Amber / White)
$g.FillEllipse($amberSatBrush, 230 - 9, 282 - 9, 18, 18)
$g.FillEllipse([System.Drawing.Brushes]::White, 230 - 4, 282 - 4, 8, 8)

$g.Dispose()

Save-MultiResIcon -MasterBitmap $bmpAfc -OutputIcoPath "D:\Aaroneous\assets\icons\afc_controller.ico" -OutputPngPath "D:\Aaroneous\assets\icons\afc_controller.png"
$bmpAfc.Dispose()

Write-Host "`nAll selected icons successfully baked into D:\Aaroneous\assets\icons!"
