FROM python:3.13

WORKDIR /backend

# RUN apt-get update && apt-get install -y libpq-dev gcc && apt-get clean
# NOTE: Might need to change this for production
# I'm considering versioning issues here
RUN apt-get update && apt-get install -y postgresql-client
RUN pip install --no-cache-dir --upgrade pip

# Use UID 1000 to match devcontainer (avoids permission issues with mounted volumes)
RUN adduser -u 1000 --disabled-password --gecos "" lifted && chown -R lifted /backend
USER lifted

COPY requirements.txt /backend/
RUN pip install -r requirements.txt
RUN echo 'alias devserver="uvicorn app.main:app --reload --host 0.0.0.0 --port 8000 --reload --log-level debug"' >> ~/.bashrc

ENV PATH="$PATH:/home/lifted/.local/bin"
COPY . /backend/

CMD ["uvicorn", "app.main:app", "--reload", "--host", "0.0.0.0", "--port", "8000", "--reload", "--log-level", "debug"]
