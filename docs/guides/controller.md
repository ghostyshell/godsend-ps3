# Use the virtual controller

The Controller page talks to webMAN MOD's `/pad.ps3` commands, so the console
sees the presses as its own pad. webMAN must be running on port 80.

## Mouse and touch

Press a key and hold it: the button lights up and stays held until you let go,
which is what a long press in the XMB needs. A quick tap is a quick press.

The d-pad and the four face buttons are laid out as a DualShock. L1, L2, L3,
R1, R2 and R3 sit along the top; Select, PS and Start along the middle. Each
analog stick has eight direction buttons.

## Keyboard

| Key | Button |
|---|---|
| Arrows | d-pad |
| Enter | Cross |
| Esc | Circle |
| Backspace | Square |
| T | Triangle |
| Q / E | L1 / R1 |
| 1 / 3 | L2 / R2 |
| Shift | L3 / R3 |
| Tab | Select |
| Space | Start |
| F | PS |

Hold a key to hold the button; release it to lift. Keys are ignored while a
modifier is down so browser shortcuts still work.

## Combos

Presets cover L3 + R3, SELECT + START, SELECT + CIRCLE and R2 + L3 + CIRCLE.
Build a combo lets you choose any set of buttons and press them together.

## Macros

A macro is a comma-separated sequence. Each step is a button or a `a+b` chord:

```
cross, down, down, cross
up, l1+r1, cross
```

Press Run macro and watch the console.

## Console actions

The same page can send the PS button, request a screenshot, or restart the
console. Screenshots appear in `/dev_hdd0/photo`.
