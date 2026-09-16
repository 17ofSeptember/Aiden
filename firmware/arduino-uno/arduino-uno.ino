// Aiden Uno acquisition firmware, protocol 1. No recognition runs here.
#include <Arduino.h>
#include <util/atomic.h>
constexpr uint8_t ADC_CHANNEL = 0; // A0
constexpr uint8_t LEAD_PLUS_PIN = 8;
constexpr uint8_t LEAD_MINUS_PIN = 9;
constexpr bool USE_LEAD_OFF = true;
constexpr uint32_t SERIAL_BAUD = 115200;
constexpr uint16_t DEFAULT_RATE = 500;
constexpr uint8_t FRAME_SIZE = 19;
constexpr uint8_t QUEUE_SIZE = 32;
struct Sample { uint32_t sequence; uint32_t time; uint16_t adc; uint8_t flags; };
volatile Sample samples[QUEUE_SIZE];
volatile uint8_t head=0,tail=0;
volatile uint32_t sequence=0;
volatile bool streaming=false,overflowed=false;
uint16_t sampleRate=DEFAULT_RATE;
uint32_t lastContact=0;
uint8_t command[FRAME_SIZE];
uint8_t commandLength=0;

uint16_t crc(const uint8_t* bytes,uint8_t n){uint16_t value=0xffff;for(uint8_t i=0;i<n;i++){value^=(uint16_t)bytes[i]<<8;for(uint8_t j=0;j<8;j++)value=(value&0x8000)?(value<<1)^0x1021:value<<1;}return value;}
void put16(uint8_t* p,uint16_t v){p[0]=v;p[1]=v>>8;}
void put32(uint8_t* p,uint32_t v){for(uint8_t i=0;i<4;i++)p[i]=v>>(8*i);}
void sendFrame(uint8_t type,uint32_t seq,uint32_t time,uint16_t value,uint8_t flags){uint8_t frame[FRAME_SIZE]={0xa5,0x5a,1,type};put32(frame+4,seq);put32(frame+8,time);put16(frame+12,value);frame[14]=flags;put16(frame+15,sampleRate);put16(frame+17,crc(frame+2,15));Serial.write(frame,FRAME_SIZE);}
void configureTimer(uint16_t rate){
  ATOMIC_BLOCK(ATOMIC_RESTORESTATE){
    TCCR1A=0;TCCR1B=0;TCNT1=0;
    OCR1A=(F_CPU/8UL/rate)-1;OCR1B=OCR1A;
    TCCR1B=_BV(WGM12)|_BV(CS11);
    ADMUX=_BV(REFS0)|ADC_CHANNEL;
    // Timer1 compare B auto-triggers ADC; ADC clock 125 kHz (104 us conversion).
    ADCSRB=_BV(ADTS2)|_BV(ADTS0);
    ADCSRA=_BV(ADEN)|_BV(ADATE)|_BV(ADIE)|_BV(ADPS2)|_BV(ADPS1)|_BV(ADPS0)|_BV(ADSC);
    TIMSK1=_BV(OCIE1B);
  }
}
ISR(TIMER1_COMPB_vect) {} // Clears trigger flag so the next compare can trigger ADC.
ISR(ADC_vect){
  uint16_t adc=ADC;uint32_t seq=sequence++;
  if(!streaming)return;
  uint8_t next=(head+1)%QUEUE_SIZE;
  if(next==tail){overflowed=true;return;}
  samples[head].sequence=seq;samples[head].time=micros();samples[head].adc=adc;
  samples[head].flags=(USE_LEAD_OFF?((digitalRead(LEAD_PLUS_PIN)?1:0)|(digitalRead(LEAD_MINUS_PIN)?2:0)):0)|(overflowed?4:0);
  overflowed=false;head=next;
}
void handleCommand(){
  if(command[2]!=1 || crc(command+2,15)!=(uint16_t)(command[17]|((uint16_t)command[18]<<8)))return;
  uint8_t type=command[3];
  if(type==0x10){
    uint16_t requested=command[15]|((uint16_t)command[16]<<8);
    if(requested!=250 && requested!=500 && requested!=1000)return;
    ATOMIC_BLOCK(ATOMIC_RESTORESTATE){streaming=false;head=tail=0;}
    sampleRate=requested;configureTimer(sampleRate);lastContact=millis();
    if(Serial.availableForWrite()>=FRAME_SIZE)sendFrame(2,0,0,0x0100,0);
  }else if(type==0x11){lastContact=millis();ATOMIC_BLOCK(ATOMIC_RESTORESTATE){head=tail=0;streaming=true;}}
  else if(type==0x12){streaming=false;lastContact=millis();}
  else if(type==0x13){lastContact=millis();}
}
void setup(){
  pinMode(LEAD_PLUS_PIN,INPUT);pinMode(LEAD_MINUS_PIN,INPUT);pinMode(LED_BUILTIN,OUTPUT);
  Serial.begin(SERIAL_BAUD);configureTimer(sampleRate);
}
void loop(){
  while(Serial.available()){
    uint8_t b=Serial.read();
    if(commandLength==0 && b!=0xa5)continue;
    if(commandLength==1 && b!=0x5a){commandLength=b==0xa5?1:0;continue;}
    command[commandLength++]=b;
    if(commandLength==FRAME_SIZE){handleCommand();commandLength=0;}
  }
  if(streaming && (uint32_t)(millis()-lastContact)>3000)streaming=false;
  digitalWrite(LED_BUILTIN,streaming?HIGH:LOW);
  if(head!=tail && Serial.availableForWrite()>=FRAME_SIZE){
    Sample s;
    ATOMIC_BLOCK(ATOMIC_RESTORESTATE){s.sequence=samples[tail].sequence;s.time=samples[tail].time;s.adc=samples[tail].adc;s.flags=samples[tail].flags;tail=(tail+1)%QUEUE_SIZE;}
    if(streaming)sendFrame(1,s.sequence,s.time,s.adc,s.flags);
  }
}
