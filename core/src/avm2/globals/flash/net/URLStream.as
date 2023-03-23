package flash.net {

	import __ruffle__.stub_method;
	import flash.net.URLRequest;
	import flash.utils.Endian;

	public class URLStream {
		private var _bytesAvailable:uint = 0;
		private var _connected:Boolean = false;
		private var _endian:String = Endian.BIG_ENDIAN;

		public function get bytesAvailable():uint {
			return _bytesAvailable;
		}

		public function get connected():Boolean {
			return _connected;
		}

		public function get endian():String {
			return this._endian;
		}
		public function set endian(value:String):void {
			if (value === Endian.BIG_ENDIAN || value === Endian.LITTLE_ENDIAN) {
				this._endian = value;
			} else {
				throw new ArgumentError("Error #2008: Parameter type must be one of the accepted values.", 2008);
			}
		}

		public function close():void {
			stub_method("flash.net.URLStream", "close");
		}

		public function load(request:URLRequest):void {
			stub_method("flash.net.URLStream", "load");
		}
	}
}
