UniWS is an automated hex editor. Once you figure out where to make edits to a file, by providing the correct paramters to UniWS it can make these edits for a person without them needing to use a hex editor.

```ini
[Apps]
   version=1.03
   
   a0=Dungeon Lords
   a1=Empire Earth II
   ...
```
- **version**: Specifies which version of UniWS this patches.ini file is for; this must match the version of the app to be used. Since UniWS is no longer being developed, this will always be 1.03
- **a#**: These entries determine the the list of games available in the Game dropdown in UniWS.

```ini
[Need for Speed: Underground 2]
   details=Patches any game version, but only works on a cracked SPEED2.EXE13101310Select the 640x480 resolution in game to use your custom resolution.
   checkfile=speed2.exe
   modfile=speed2.exe
   undofile=speed2.undo1
   sig=80020000C701E0010000
   sigwild=0000110000
   xoffset=0
   yoffset=6
   occur=1
```

Here's what each setting means:
- **\[game name\]**: This identifies a group of settings for a game. The name in brackets must match a string provided for the a# setting in the header section to be used. All settings under a bracketed entry are treated as part of the same entry until the next bracketd entry.
- **details**: Text note that appears in the "Important Details" text box in the UniWS GUI. For line breaks in the display use the carriage return/line feed ASCII values: "1310" Do not put any actual line breaks in the description except at the end. 
- **checkfile**: UniWS looks for the presence of this file in order to verify that the user has selected the correct directory for the game. Typically you would use the name of the file you need to modify, but it may be the case that the file you need to modify has a generic name used by other applications. In this case you should use a different checkfile that is unique to the game you are modifying.
- **modfile**: The filename of the file you need to modify.
- **undofile**: UniWS has the ability to undo the edits it makes to the modfile, it automatically saves the information necessary to undo the changes in the undofile. This may be any filename of your choice; the precedent is to use the modfile filename with a .undoX extension where X is the number of the edit (only important when multiple edits are made). The undo files will be placed in the same directory as the modfile.
- **sig**: This hex string is used to uniquely identifies where the edit is to be made. UniWS will search the modfile for a match to this string (also dependent on sigwild, see below) and place the internal "edit cursor" at the starting position of this string. Must be a set of bytes (one byte is two hex digits, so in other words, it must be an even number of digits in length). There is no practical upper or lower limit on the number of bytes in the sig. The string need be only as long as required to uniquely identify the string you need to edit in the file.
- **sigwild**: Bit flags that indicate whether a particular byte in the sig string is to be treated as a wildcard when locating the matching string in the modfile. 1 indicates a byte is a wildcard, 0 indicates it must be matched exactly. You must have a sigwild flag for all bytes in the sig string, even if you have no wildcard bytes.

> [!NOTE]
> In the example: The 5th and 6th bytes (C7 01) are wildcard bytes - their value doesn't actually matter, they are in the sig string merely as placeholders to indicate the number of bytes between "known" strings. This means that UniWS will search the modfile for the hex string "80020000", followed by any two bytes, followed by "E0010000".

- **xoffset**/**yoffset**: Appropriately enough these specify the offset, in number of bytes, from the beginning of the sig string where to write the user defined resolution value. xoffset specifies where the width value is written, yoffset the height value. The offset is 0-based, so the first byte is 0, the second byte is 1, etc. UniWS will always write 1 word (2 bytes) starting from the offset for a resolution value.

> [!NOTE]
> In the example: The user entered width value will overwrite the 1st and 2nd bytes (80 02) in the sig string; the height value will overwrite the 7th and 8th bytes (E0 01).

- **occur**: The number of occurrences of the hex string to be edited in the file. UniWS will update this number of occurrences of the hex string sequentially, starting from the beginning of the file.

```ini
[Star Wars: KOTOR (800x600 interface)]
   details=(removed for brevity)
   checkfile=swkotor.exe
   modfile=swkotor.exe
   undofile=swkotora.undo1
   sig=3D20030000EFEFEFEFEFEF58020000
   sigwild=000001111110000
   xoffset=1
   yoffset=11
   occur=1
   ;interface mods
   ;1024
   p1modfile=swkotor.exe
   p1undofile=swkotora.undo2
   p1sig=3D00040000B329EFEFEFEFEFEFEFEFEF3D00050000EFEF3D40060000
   p1sigwild=0000000111111111000001100000
   p1xoffset=1
   p1occur=1
   p1setx=0
```
- **;comment**: Semi-colon merely designates a comment, anything after a semi-colon until the next line break will be ignored.
- **p#setting**: When you need to make multiple hex edits for a game, you merely add an additional group of the settings described above and give them a prefix of p#,where # is replaced with an appropriate integer for additional sets. Note that each modification set must have it's own modfile (even if it's the same) and undofile specified. You can modify more than one file for the same game by specifying a different modfile. An additionalcheckfile should not be specified. As far as I know, there is no limit to the amount of edit sets you can have.
- **setx**/**sety**: A hardcoded value to set. When provided, UniWS will replace the x or y words in the hex string with this value instead of using the value from the resolution that the user enters. 
