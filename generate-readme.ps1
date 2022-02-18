$ErrorActionPreference = "Stop"

Push-Location $PSScriptRoot
try {
    (Get-Content "README.tpl" -Raw) `
        -replace "{{intro}}", (Get-Content "doc/intro.md" -Raw) `
        -replace "{{decoding}}", (Get-Content "doc/decoding.md" -Raw) `
        -replace "{{json}}", (Get-Content "doc/json.md" -Raw) `
        -replace "{{reflect}}", (Get-Content "doc/reflect.md" -Raw) `
    | Set-Content "README.md"
}
finally { Pop-Location }