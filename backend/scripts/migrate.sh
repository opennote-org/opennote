#!/usr/bin/env sh

main() {
    # Launch this at `backend` root after done writing the migration script. 
    # It will update the tables in the database. 
    # You need to do this before generating entities. 
    sea-orm-cli migrate \
        --migration-dir "./crates/migration" \
        --database-url sqlite://./data/database.sqlite?mode=rwc
}

main "$@"