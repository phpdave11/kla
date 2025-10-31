# Configuration

Kla searches for the "main" configuration file in the following places:

- `config.toml`
- `~/.kla.toml`
- `~/.config/kla/config.toml` _prefered_
- `/etc/kla/config.toml`

The first file that it finds will be selected and parsed. If there is an error during parsing kla will return that error and stop executing.

## Additional Configuration Files

Configuration can be broken into multiple files! Your "main" config, **and only your main config**, can specify a `[[config]]` attribute which pulls in additional directories or paths.

```toml
[[config]]
path = "/etc/kla/elasticsearch_environment.toml"

[[config]]
path = "/etc/kla/ntfy.toml"
```

You can also specify a directory!

```toml
[[config]]
dir = "/etc/kla/conf.d"
```

