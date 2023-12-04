### Description
Rust-powered blazingly-fast HTTP Axum backend service for 'Plexhelp'. Meant to be used together with 'Front' (Angular) project.

### Setup
Copy proviede user `videosmover.service` systemd file to `~/.config/systemd/user`, then reload systemd user daemon with:
`systemctl --user daemon-reload`  

Then enable service to start on startup with:
`systemctl --user enable videosmover.service`

### Config
Sample config file is located in `./config` directory.  
Adjust and copy it inside a `config` folder in your server deployment directory

### Build and install
Run provided `./install.sh` file.  
It will run tests, build server, stop running service (if any), install new server and restart service.  

To start or stop service manually use:  
`systemctl --user start|stop videosmover.service`

### OpenAPI
For communicating through a known contract with other services, use provided OpenAPI `spec.yml` file.   

Spec can be regenerated with cargo command:   
`cargo run --bin gen_openapi`   

SwaggerUI is also provided at following URI of running server:   
`/swagger-ui`
