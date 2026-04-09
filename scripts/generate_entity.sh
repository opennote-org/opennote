#!/usr/bin/env sh

# NOTICE
# 1. You should run `./scripts/migrate.sh` before generating entities
# 2. Run this from the project root

main() {
	# run this script at the `backend`'s root,
	# when wanting to create entities to reflect the sql data structure changes
	
	ENV_PATH="scripts/.env"
	
	# Load .env file if it exists
    if [ -f $ENV_PATH ]; then
        set -a
        source $ENV_PATH
        set +a
        echo ".env loaded successfully"
    else
        echo ".env missing. You should have an .env file located in the ./script folder"
        exit 1
    fi

    sea-orm-cli generate entity \
        --database-url $SQLITE_DATABASE \
        --output-dir ./crates/opennote-entities/src \
        --entity-format dense
}

main "$@"
