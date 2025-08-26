String buf = "";
String parts[2];

void setup() {
  Serial.begin(9600);
  while (!Serial) {
    ;
  }
}

void loop() {
  if (Serial.available() == 0) {
    return;
  }

  char incoming = Serial.read();
  buf += incoming;

  if (incoming == '\n') {
    buf.trim();
    splitString(buf);

    int pin = parts[1].toInt();

    bool value;
    if (parts[0] == "on") {
      value = true;
    } else if (parts[0] == "off") {
      value = false;
    } else {
      error();
      return;
    }

    if (pin == 0) {
      error();
      return;
    } else if (pin > 1 && pin < 14) {
      pinMode(pin, OUTPUT);
      digitalWrite(pin, value);
    } else {
      error();
      return;
    }

    buf = "";
    Serial.print("success");
  }
}

void error() {
  Serial.print("what is a '");
  Serial.print(buf);
  Serial.print("'\n");
  
  buf = "";
}

void splitString(String input) {
  int spaceIndex = input.indexOf(' ');

  if (spaceIndex != -1) {
    parts[0] = input.substring(0, spaceIndex);
    parts[1] = input.substring(spaceIndex + 1);
  } else {
    parts[0] = input;
    parts[1] = "";
  }
}
