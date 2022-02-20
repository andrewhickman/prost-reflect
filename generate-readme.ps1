$ErrorActionPreference = "Stop"

Push-Location $PSScriptRoot
try {
    (Get-Content "README.tpl" -Raw) `
        -replace "{{intro}}", (Get-Content "prost-reflect/doc/intro.md" -Raw) `
        -replace "{{decoding}}", (Get-Content "prost-reflect/doc/decoding.md" -Raw) `
        -replace "{{json}}", (Get-Content "prost-reflect/doc/json.md" -Raw) `
        -replace "{{reflect}}", (Get-Content "prost-reflect/doc/reflect.md" -Raw) `
    | Set-Content "README.md"
}
finally { Pop-Location }