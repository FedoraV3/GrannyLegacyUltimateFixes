# The Granny Legacy Ultimate Fixes
This is my recent project yet, granny legacy ultimate fixes.

It will include:

- No Steam Requirement
- Optimizations
- Seed Limit Remover (Integer limit up to 64bit)
- More extra options
- .. And more in the future!

As of GMT+8, October 9, 2026, 9:05pm, at the time of writing,
this mod is still in the works! Come back in a month to see
the progress that has been made on this mod!

# Building
Standard steps first:

>git clone https://github.com/FedoraV3/GrannyLegacyUltimateFixes

>cd /path/to/GrannyLegacyUltimateFixes

>cargo build --release

Go to /path/to/GrannyLegacyUltimateFixes/target/release,
Rename GrannyLegacyUltimateFixes.dll to GrannyLegacyUltimateFixes.asi,
then follow the installation steps below.

# Installation
[Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader)
Download [version.dll](https://github.com/ThirteenAG/Ultimate-ASI-Loader/releases/download/x64-latest/version-x64.zip) and drag it to granny legacy root folder (Where .exe is located).
And then download the .asi file and drag it in the root folder also (Where you just dragged the version.dll)
Open the game and it should be done

# How the mod will be loaded
This time, we will use [Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader)
to load this mod, if you have any 
other mod loader this may conflict, 
but I make native dlls for windows,
not dlls for melonloader/bepinex.

# Notes
- I will be using the optimization from my other project, [GrannyQOLMod](https://github.com/FedoraV3/GrannyQOLMod) but refined.
- Most of the features will be re-used in the first iteration of the project, but more unique features will be added, as I want to combine all my small projects into one ultimate project.
- The mod is required to be loaded before the game intro ends, otherwise it will bug! Which is why I used a modloader for ease of use.
- I have not used any offsets to get anything. I will make this mod able to survive updates with almost no maintenance. If it breaks, I will fix it.