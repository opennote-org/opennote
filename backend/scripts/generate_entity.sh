#!/usr/bin/env sh

main() {
	# run this script at the `backend`'s root
sea-orm-cli generate entity \
    --database-url sqlite://./data/database.sqlite?mode=rwc \
    --output-dir ./src/database/entity \
    --entity-format dense
}

main "$@"
