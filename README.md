# Karlsruhe Garbage Calendar

Take a look at <https://web6.karlsruhe.de/service/abfall/akal/akal.php>. Isn't it nice to check this page every few weeks because of irregularities in the recurring collection dates? This repository tries to remedy that problem by providing an iCalendar server.

## Usage

### Server

The server binds to the port `8008`.
You can get your garbage collection date calendar with the path `/calendar?street=<your_street>&street_number=<your_street_number>`.

You can exclude specific waste types with the following query parameters:
- `exclude_residual`
- `exclude_organic`
- `exclude_recyclable`
- `exclude_paper`
- `exclude_bulky`

The value of these parameters must be `true` or `false`.
By default, no waste types are excluded.

### CLI

The application can also be started with the subcommand `cli` to just get and write the calendar to the file `calendar.ics` in the current working directory.

## Contributing

Write a strongly worded letter to the city administration of Karlsruhe to provide this functionality themselves.
