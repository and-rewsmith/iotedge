FROM ubuntu:18.04

ARG EXE_DIR=.

WORKDIR /app

COPY $EXE_DIR/generic_mqtt_tester ./

# Add an unprivileged user account for running the module
USER moduleuser
CMD echo "$(date --utc +"[%Y-%m-%d %H:%M:%S %:z]"): Starting Module" && \
generic_mqtt_tester