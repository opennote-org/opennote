#!/usr/bin/env sh

# NOTICE
# 1. You need to do this before generating entities. 
# 2. Run this from the project root

main() {
    # Launch this at `backend` root after done writing the migration script. 
    # It will update the tables in the database. 
    # 
    # TODO: need to separate the debug database and the prod database
    
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
    
    sea-orm-cli migrate \
        --migration-dir "./crates/opennote-data/crates/migration" \
        --database-url $SQLITE_DATABASE
}

main "$@"