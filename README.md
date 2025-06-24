# kybe's minecraft server scanner

## don't forget to star this repo if you like it :-)

# SETUP
## DB
- install [postgresql](https://www.postgresql.org/)
- create a user (i recommend mc_scanner)
- cratea a db (i recommend mc_scanner)
- make sure the user has full acess over that db and is allowed to remotly log in
## Scanner
- edit the config
- ip_range is the range to be scanned (0.0.0.0/0 is all ipv4s)
- masscan_rate is masscans rate (see masscan docs for more info about this)
- isp_scan_enabled is going to do more scanning of ports / ips (it does a closer scan on the /24 block when a ip is found)
- mc_checker_threads is how many checker threads should run (this only affects the scanner part. not the masscan part)
- database_url is the url to the db ("host=127.0.0.1 port=5555 user=mc_scanner password=pwd dbname=mc_scanner") replace host, port, user, password and the dbname as needed
- timeout_ms is how long the checker is going to allow a server to respond before skipping it
- blacklist_file is used as blacklist for masscan
- masscan_use_sudo runs masscan as sudo (requires manual password input) (you can also run it as root from the start and disable this)
## Client
- press = to open the gui
- use the arrow or wasd keys to move around the gui
- when being on the "Scanner Accessor" press d or right mouse button
- go to "Database URL: null" and press d or right mouse button then enter your url in the format of "jdbc:postgresql://localhost:5555/mc_scanner"
- same for user and pass
- after that you can edit the query it will add every ip it gets as response (ip column)
- enable the module to add them to the server list
- enable Clear Servers to clear the server list

# FAQ
**why is my cpu at 100%?**
1. you have to many workers in the config (rare)
2. you have reached the file limit on linux (windows to but fuck windows)
   you can fix this by running ulimit -n 100000 (allows up to 100000 files)

![image](https://github.com/user-attachments/assets/e4dc0316-1806-45bc-860e-3abfbd8a21f2)
