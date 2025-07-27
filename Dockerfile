
# Use a slim Python image as the base
FROM python:3.10-slim

# Install sympy
RUN pip install sympy

# Set the working directory
WORKDIR /usr/src/app

# Copy the script into the container
COPY . .

# The command to run when the container starts
CMD ["python", "./script.py"]

