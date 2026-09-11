#!/usr/bin/env python3
"""Build the standalone GitHub Pages designer; no npm install or server needed."""
from pathlib import Path
import json
import shutil
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / 'tools/syntax-designer'
OUT = ROOT / 'docs/designer'
SAMPLES = {
'python': '''# A small welcome, made a little more personal.
from dataclasses import dataclass

@dataclass
class Greeting:
    name: str
    favorites: list[str]

    def message(self) -> str:
        return f"Hello, {self.name}!"

visitor = Greeting("Ada", ["quiet tools", "good coffee"])

if visitor.favorites:
    print(visitor.message())
    print("Make something that feels like you.")
''',
'csharp': '''// A small welcome, in C#.
using System;

public record Greeting(string Name)
{
    public string Message() => $"Hello, {Name}!";
}

var visitor = new Greeting("Ada");
var enabled = true;

if (enabled)
{
    Console.WriteLine(visitor.Message());
}
''',
'c': '''/* A small welcome, in C. */
#include <stdio.h>
#define MAX_GUESTS 32

int main(void) {
    const char *name = "Ada";
    int visits = 1;

    if (visits < MAX_GUESTS) {
        printf("Hello, %s!\\n", name);
    }
    return 0;
}
''',
'cpp': '''// A small welcome, in C++.
#include <iostream>
#include <string>

class Greeting {
public:
    explicit Greeting(std::string name) : name_(name) {}
    void show() const {
        std::cout << "Hello, " << name_ << "!";
    }
private:
    std::string name_;
};
''',
'php': '''<?php
// A small welcome, in PHP.
namespace App;

final class Greeting
{
    public function __construct(private string $name) {}

    public function message(): string
    {
        return "Hello, {$this->name}!";
    }
}

$visitor = new Greeting("Ada");
echo $visitor->message();
''',
'rs': '''// A small welcome, in Rust.
#[derive(Debug)]
struct Greeting {
    name: String,
    visits: u32,
}

fn main() {
    let visitor = Greeting {
        name: String::from("Ada"),
        visits: 1,
    };
    println!("Hello, {}!", visitor.name);
}
''',
'javascript': '''// A small welcome, in JavaScript.
const visitor = {
  name: "Ada",
  favorites: ["quiet tools", "good coffee"],
};

function greet(person) {
  const message = `Hello, ${person.name}!`;
  return message;
}

if (visitor.favorites.length > 0) {
  console.log(greet(visitor));
}
''',
 'typescript': '''// A small welcome, in TypeScript.
interface Visitor {
  name: string;
  favorites: string[];
}

function greet(person: Visitor): string {
  return `Hello, ${person.name}!`;
}

const visitor: Visitor = {
  name: "Ada",
  favorites: ["quiet tools", "good coffee"],
};
console.log(greet(visitor));
''',
'go': '''// A small welcome, in Go.
package main

import "fmt"

type Visitor struct {
    Name string
    Visits int
}

func main() {
    visitor := Visitor{Name: "Ada", Visits: 1}
    fmt.Printf("Hello, %s!\\n", visitor.Name)
}
''',
'java': '''// A small welcome, in Java.
public class Greeting {
    private final String name;

    public Greeting(String name) {
        this.name = name;
    }

    public String message() {
        return "Hello, " + name + "!";
    }
}
''',
'shell': '''#!/bin/sh
# A small welcome, from the shell.
name="Ada"
visits=1

if [ "$visits" -gt 0 ]; then
    printf 'Hello, %s!\\n' "$name"
fi

for favorite in "quiet tools" "good coffee"; do
    echo "$favorite"
done
''',
'lua': '''-- A small welcome, in Lua.
local visitor = {
    name = "Ada",
    visits = 1,
}

local function greet(person)
    return "Hello, " .. person.name .. "!"
end

if visitor.visits > 0 then
    print(greet(visitor))
end
''',
'json': '''{
  "name": "Ada",
  "greeting": "Make something that feels like you.",
  "favorites": [
    "quiet tools",
    "good coffee"
  ],
  "visits": 1,
  "enabled": true,
  "lastVisit": null
}
''',
 'toml': '''# A few settings, in TOML.
[visitor]
name = "Ada"
visits = 1
enabled = true
favorites = ["quiet tools", "good coffee"]

[appearance]
background = "#161817"
accent = "#a3e3b5"
''',
'yaml': '''# A few settings, in YAML.
visitor:
  name: "Ada"
  visits: 1
  enabled: true
  favorites:
    - quiet tools
    - good coffee

appearance:
  background: "#161817"
  accent: "#a3e3b5"
''',
'html': '''<!DOCTYPE html>
<html lang="en">
  <head>
    <title>A small welcome</title>
  </head>
  <body>
    <!-- Make yourself at home. -->
    <main class="greeting">
      <h1>Hello, Ada!</h1>
      <p>Quiet tools &amp; good coffee.</p>
    </main>
  </body>
</html>
''',
'css': '''/* A small welcome, styled your way. */
:root {
  --accent: #a3e3b5;
}

.greeting {
  color: #edf3ec;
  background: #161817;
  padding: 24px;
  border-radius: 12px;
}

@media (max-width: 600px) {
  .greeting { padding: 16px; }
}
''',
'sql': '''-- A small welcome, in SQL.
CREATE TABLE visitors (
    id INTEGER PRIMARY KEY,
    name VARCHAR(80) NOT NULL,
    visits INTEGER DEFAULT 1
);

INSERT INTO visitors (name) VALUES ('Ada');

SELECT name, visits
FROM visitors
WHERE visits > 0
ORDER BY name ASC;
''',
}

def main():
    locks = [tomllib.loads(path.read_text())['package'] for path in [ROOT / 'Cargo.lock', TOOL / 'Cargo.lock']]
    versions = [{p['name']: p['version'] for p in lock} for lock in locks]
    for name in ('regex', 'regex-automata', 'regex-syntax', 'toml', 'serde'):
        if versions[0][name] != versions[1][name]:
            raise SystemExit(f'{name} differs from the desktop lockfile; align dependencies before building.')
    if '--assets-only' not in sys.argv:
        subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(TOOL / 'Cargo.toml'),
                        '--target', 'wasm32-unknown-unknown', '--target-dir', str(TOOL / 'target'), '--release', '-j', '2'], cwd=ROOT, check=True)
    OUT.mkdir(exist_ok=True)
    shutil.copyfile(TOOL / 'target/wasm32-unknown-unknown/release/potyi_syntax_designer.wasm', OUT / 'engine.wasm')
    presets = []
    for path in sorted((ROOT / 'config/syntax').glob('*.toml')):
        language = path.stem.removeprefix('syntax_')
        presets.append({'filename': path.name, 'definition': tomllib.loads(path.read_text())['syntax'], 'sample': SAMPLES[language]})
    presets.append({'filename':'syntax_custom.toml', 'definition':{'name':'Custom', 'extensions':['example'], 'rules':[
        {'name':'comment', 'pattern':'#.*$', 'color':'#6A9955'},
        {'name':'keyword', 'pattern':r'\b(begin|end)\b', 'color':'#569CD6'},
    ]}, 'sample':'# A language of your own.\nbegin\n    example\nend\n'})
    (OUT / 'presets.json').write_text(json.dumps(presets, ensure_ascii=False, indent=2) + '\n')
    print(f'Built {len(presets)} designer presets and engine.wasm ({(OUT / "engine.wasm").stat().st_size:,} bytes).')

if __name__ == '__main__': main()
