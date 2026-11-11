# Builds webapp/root_files/vocal-synth-en-us.dict, the English pronouncing dictionary the text-to-speech page reads,
# from the CMU Pronouncing Dictionary (BSD-style license, kept at the top of the output file).
#
# The download is pinned to one commit, so the output only changes when $Commit does. Commit the output, the same as
# vocal-synth.wasm: the tests and the deploy use the committed file, and nothing rebuilds it.
#
# The file format is read by synth/src/dictionary.rs, which documents it:
#   - Lines starting with ';' are comments (the license).
#   - '!codes' lists the one-character code for each sound, as code=IPA.
#   - Every other line is one word and its pronunciations, sorted by word. The word starts with a digit, the number
#     of characters it shares with the word before it (front coding), then the rest of the word, a space, and its
#     pronunciations separated by '|'. A vowel's code is followed by its stress: 0, 1, or 2.
param(
    [string]$Commit = '74790861f652b15e4ac49015a90074ad62a27690'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$out = Join-Path $root 'webapp/root_files/vocal-synth-en-us.dict'
$base = "https://raw.githubusercontent.com/cmusphinx/cmudict/$Commit"

# ARPAbet to IPA, with one code each. Vowels are upper case, consonants lower case. AH and ER have their own codes
# when unstressed (/ə/ and /ɚ/), matching ARPABET in synth/src/english.rs.
$sounds = [ordered]@{
    'AA' = @('A', 'ɑ'); 'AE' = @('E', 'æ'); 'AH' = @('U', 'ʌ'); 'AH0' = @('X', 'ə'); 'AO' = @('O', 'ɔ')
    'AW' = @('W', 'aʊ'); 'AY' = @('Y', 'aɪ'); 'EH' = @('H', 'ɛ'); 'ER' = @('R', 'ɝ'); 'ER0' = @('Q', 'ɚ')
    'EY' = @('J', 'eɪ'); 'IH' = @('I', 'ɪ'); 'IY' = @('L', 'i'); 'OW' = @('B', 'oʊ'); 'OY' = @('C', 'ɔɪ')
    'UH' = @('V', 'ʊ'); 'UW' = @('K', 'u')
    'B' = @('b', 'b'); 'CH' = @('c', 'tʃ'); 'D' = @('d', 'd'); 'DH' = @('q', 'ð'); 'F' = @('f', 'f')
    'G' = @('g', 'ɡ'); 'HH' = @('h', 'h'); 'JH' = @('j', 'dʒ'); 'K' = @('k', 'k'); 'L' = @('l', 'l')
    'M' = @('m', 'm'); 'N' = @('n', 'n'); 'NG' = @('x', 'ŋ'); 'P' = @('p', 'p'); 'R' = @('r', 'ɹ')
    'S' = @('s', 's'); 'SH' = @('w', 'ʃ'); 'T' = @('t', 't'); 'TH' = @('o', 'θ'); 'V' = @('v', 'v')
    'W' = @('y', 'w'); 'Y' = @('i', 'j'); 'Z' = @('z', 'z'); 'ZH' = @('e', 'ʒ')
}

function Convert-Pronunciation([string[]]$phones) {
    $sb = [System.Text.StringBuilder]::new()
    foreach ($phone in $phones) {
        $stress = ''
        $symbol = $phone
        if ($phone -match '^([A-Z]+)([012])$') {
            $symbol = $Matches[1]
            $stress = $Matches[2]
        }
        $key = if ($stress -eq '0' -and $sounds.Contains("${symbol}0")) { "${symbol}0" } else { $symbol }
        if (-not $sounds.Contains($key)) { throw "Unknown ARPAbet symbol '$phone'" }
        [void]$sb.Append($sounds[$key][0]).Append($stress)
    }
    $sb.ToString()
}

$temp = Join-Path ([System.IO.Path]::GetTempPath()) "vocalbin-cmudict-$Commit"
New-Item -ItemType Directory -Force -Path $temp | Out-Null
$dictFile = Join-Path $temp 'cmudict.dict'
$licenseFile = Join-Path $temp 'LICENSE'
if (-not (Test-Path $dictFile)) { Invoke-WebRequest "$base/cmudict.dict" -OutFile $dictFile }
if (-not (Test-Path $licenseFile)) { Invoke-WebRequest "$base/LICENSE" -OutFile $licenseFile }

# Word to its pronunciations, in the dictionary's order, without repeats.
$words = [System.Collections.Generic.Dictionary[string, System.Collections.Generic.List[string]]]::new([System.StringComparer]::Ordinal)
foreach ($line in [System.IO.File]::ReadLines($dictFile)) {
    $line = ($line -split '#', 2)[0].Trim()
    if ($line -eq '') { continue }
    $parts = $line -split '\s+'
    $word = $parts[0] -replace '\(\d+\)$', ''
    if ($word -cnotmatch "^[a-z0-9'.\-]+$") { throw "Unexpected word '$word'" }
    $pronunciation = Convert-Pronunciation $parts[1..($parts.Length - 1)]
    if (-not $words.ContainsKey($word)) { $words[$word] = [System.Collections.Generic.List[string]]::new() }
    if (-not $words[$word].Contains($pronunciation)) { $words[$word].Add($pronunciation) }
}

$sorted = [string[]]$words.Keys
[System.Array]::Sort($sorted, [System.StringComparer]::Ordinal)

$sb = [System.Text.StringBuilder]::new()
[void]$sb.Append("; Vocal Bin's English pronouncing dictionary, built by PowerShell/BuildDictionary.ps1 from the CMU Pronouncing`n")
[void]$sb.Append("; Dictionary, https://github.com/cmusphinx/cmudict, commit $Commit.`n;`n")
foreach ($line in [System.IO.File]::ReadLines($licenseFile)) {
    [void]$sb.Append(('; ' + $line).TrimEnd()).Append("`n")
}
$codes = ($sounds.Values | ForEach-Object { "$($_[0])=$($_[1])" }) -join ' '
[void]$sb.Append("!codes $codes`n")
$previous = ''
foreach ($word in $sorted) {
    $shared = 0
    $limit = [math]::Min(9, [math]::Min($previous.Length, $word.Length))
    while ($shared -lt $limit -and $previous[$shared] -ceq $word[$shared]) { $shared++ }
    [void]$sb.Append($shared).Append($word.Substring($shared)).Append(' ').Append(($words[$word] -join '|')).Append("`n")
    $previous = $word
}
[System.IO.File]::WriteAllText($out, $sb.ToString(), [System.Text.UTF8Encoding]::new($false))
$size = [math]::Round((Get-Item $out).Length / 1KB)
Write-Host "Built webapp/root_files/vocal-synth-en-us.dict ($($sorted.Length) words, $size KB)" -ForegroundColor Green
