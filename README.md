# kybe's minecraft server scanner

## don't forget to star this repo if you like it :-)

# SETUP
- copy the example config to `config.toml` and change it if needed
- cargo r --release
- and done

the programm is going to setup the db on its own :-)

# FAQ
**why is my cpu at 100%?**
1. you have to many workers in the config (rare)
2. you have reached the file limit on linux (windows to but fuck windows)
   you can fix this by running ulimit -n 100000 (allows up to 100000 files)

![image](https://github.com/user-attachments/assets/e4dc0316-1806-45bc-860e-3abfbd8a21f2)
