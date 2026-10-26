# typed: strong

module Acme
  module Models
    class WidgetListResponse < Acme::Internal::Type::BaseModel
      OrHash = T.type_alias { T.any(Acme::Models::WidgetListResponse, Acme::Internal::AnyHash) }

      sig { returns(T::Array[Acme::Widget]) }
      attr_accessor :data

      sig { returns(T.nilable(String)) }
      attr_accessor :next_cursor

      sig { params(data: T::Array[Acme::Widget::OrHash], next_cursor: T.nilable(String)).returns(T.attached_class) }
      def self.new(data:, next_cursor: nil)
      end

      sig { override.returns({ data: T::Array[Acme::Widget], next_cursor: T.nilable(String) }) }
      def to_hash
      end
    end
  end
end
