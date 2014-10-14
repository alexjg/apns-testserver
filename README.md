#APNS Test Server

A simple server which parses APNS requests and exposes a ReST interface to notifications it has received.


##Building

Install cargo and then run

`cargo build`

##Usage:

```
Options:
    -h --anps-host APNS SERVER
                        The ip address the apns server will be available on,
                        default 127.0.0.1
    -p --apns-port APNS PORT
                        The port the apns server will be available on, default
                        9123
    -n --notification-server-ip HTTP SERVER
                        The ip address the notification server http interface
                        will bind to, default 127.0.0.1
    -N --notification-server-port HTTP PORT
                        The port the notification server http interface will
                        bind to, default 8080
    --cert-path SSL CERT
                        Path to the ssl certificate to use
    --private-key-path SSL PRIVATE KEY
                        Path to the ssl private key to use
```


##Rest Interface
GET requests to the root of the http interface will return a list of notifications.
DELETE requests will delete all notifications received so far.
