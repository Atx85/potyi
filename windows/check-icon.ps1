# Check the built PE resources, without executing the application.
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PotyiIconResources {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryExW(string path, IntPtr file, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool FreeLibrary(IntPtr module);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr FindResourceW(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")]
    public static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint SizeofResource(IntPtr module, IntPtr resource);
}
'@

function Read-Resource([IntPtr]$Module, [int]$Id, [int]$Type) {
    $resource = [PotyiIconResources]::FindResourceW($Module, [IntPtr]$Id, [IntPtr]$Type)
    if ($resource -eq [IntPtr]::Zero) { throw "Missing icon resource: type $Type, ID $Id" }
    $size = [PotyiIconResources]::SizeofResource($Module, $resource)
    $loaded = [PotyiIconResources]::LoadResource($Module, $resource)
    $data = [PotyiIconResources]::LockResource($loaded)
    if ($size -eq 0 -or $data -eq [IntPtr]::Zero) { throw 'Unreadable icon resource' }
    $bytes = [byte[]]::new($size)
    [Runtime.InteropServices.Marshal]::Copy($data, $bytes, 0, $bytes.Length)
    return ,$bytes
}

$path = (Resolve-Path -LiteralPath $Executable).Path
$module = [PotyiIconResources]::LoadLibraryExW($path, [IntPtr]::Zero, 2)
if ($module -eq [IntPtr]::Zero) { throw "Could not inspect executable: $path" }
try {
    $source = [IO.File]::ReadAllBytes((Join-Path $PSScriptRoot 'potyi.ico'))
    $count = [BitConverter]::ToUInt16($source, 4)
    $group = Read-Resource $module 1 14 # RT_GROUP_ICON
    if ($group.Length -ne (6 + 14 * $count) -or
        [BitConverter]::ToUInt16($group, 2) -ne 1 -or
        [BitConverter]::ToUInt16($group, 4) -ne $count) {
        throw 'Executable icon group does not match potyi.ico'
    }
    for ($i = 0; $i -lt $count; $i++) {
        $entry = 6 + 16 * $i
        $size = [BitConverter]::ToUInt32($source, $entry + 8)
        $offset = [BitConverter]::ToUInt32($source, $entry + 12)
        $id = [BitConverter]::ToUInt16($group, 6 + 14 * $i + 12)
        $actual = Read-Resource $module $id 3 # RT_ICON
        $expected = [byte[]]$source[$offset..($offset + $size - 1)]
        if ([Convert]::ToBase64String($actual) -ne [Convert]::ToBase64String($expected)) {
            throw "Embedded icon image $i does not match potyi.ico"
        }
    }
    Write-Host "Verified $count embedded Potyi icon sizes in $path"
} finally {
    [void][PotyiIconResources]::FreeLibrary($module)
}
